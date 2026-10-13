"""
OCR 引擎：RapidOCR 懒加载 + 行级结果输出 + 多语言识别。

============================================================
 概念澄清（最容易糊的一点）
============================================================
  · PP-OCRv4 = 百度的**模型体系**（det / cls / rec 三段权重）
  · ONNX 版  = paddle2onnx 导出，脱离 PaddlePaddle 大框架
  · RapidOCR = **推理封装引擎**（不是模型本身），默认即加载 PP-OCRv4 的 ONNX 权重

 结论：不是二选一，而是「RapidOCR 的执行管线 + PP-OCRv4 的 ONNX 权重」。

============================================================
 多语言识别的实现要点
============================================================
  · det / cls 两段与语言无关，全语言共用；只有 rec（识别）段与语言相关。
  · 因此「多语言 = 换 rec 权重」：不同语言按各自 rec 模型分组加载引擎。
  · 内置（en / ch_sim）走 wheel 默认权重（ch_PP-OCRv4 覆盖中英），其余语言
    从 models/ocr/<code>/ 下读对应 rec 模型 + 字典。
  · 可用语言按**磁盘上 rec 模型是否真实存在**动态判定——下载/删除语言后
    无需重启即可生效（懒加载 + 按组缓存）。

============================================================
 抽象层的意义
============================================================
 `ocr_frame()` 是本模块对外的唯一接口：输入 ndarray，返回行级结构化结果。
 将来换引擎（Paddle 原生 / PP-v5 / 自训 ONNX）只改这一层，
 **ZMQ 与 proto 完全不动**。
"""

from __future__ import annotations

import json
import os

import cv2
import numpy as np
from config import DEFAULT_OCR_LANGS, load_ocr_manifest

# 引擎标识随 FrameResult 一起落库：将来换引擎时，能据此区分「这批文本是谁认的」，
# 便于对比与回退。换引擎只需改这一处常量。
OCR_ENGINE_NAME = "rapidocr"
# 模型标识：记录权重体系，便于日后按模型维度评估识别质量
OCR_MODEL_NAME = "ppocrv4"


class OCRManager:
    """
    RapidOCR 引擎管理。

    按语言分组建引擎并缓存：det/cls 全语言通用，只有 rec 权重随语言变化。
    - en / ch_sim 共用 wheel 默认权重（一个引擎，group="default"）；
    - 其余语言各用一个 rec 模型（group=语言码），从 models/ocr/<code>/ 加载；
    - 缺失 rec 模型的语言只提示、不失败：系统有感知，但不替用户中断流程。
    """

    def __init__(self, model_storage_dir: str):
        """
        Args:
            model_storage_dir: OCR 模型目录（models/ocr/）。下载/内置的多语言
                rec 模型按 `model_storage_dir/<code>/<rec_model>` 布局存放。
        """
        self.model_storage_dir = model_storage_dir
        # 引擎缓存：key = 分组键（"default" 或语言码），value = RapidOCR 实例
        self._engines: dict[str, object] = {}
        # 语言目录：{code: {native, tier, deletable, rec_model, rec_dict, ...}}
        self._catalog: dict[str, dict] = self._load_catalog()

    # ------------------------------------------------------------------
    # 语言目录与可用性判定
    # ------------------------------------------------------------------
    def _load_catalog(self) -> dict[str, dict]:
        raw = load_ocr_manifest()
        catalog: dict[str, dict] = {}
        for lang in raw.get("languages", []):
            code = lang.get("code")
            if code:
                catalog[code] = lang
        return catalog

    def available_languages(self) -> set[str]:
        """
        动态判定当前可用的 OCR 语言：
        - builtin_wheel 恒可用（权重在 wheel 内，无需磁盘文件）；
        - builtin_bundle / downloadable 需 rec 模型文件真实存在。
        """
        available: set[str] = set()
        for code, lang in self._catalog.items():
            tier = lang.get("tier")
            if tier == "builtin_wheel":
                available.add(code)
            elif tier in ("builtin_bundle", "downloadable"):
                if self._model_files_present(code, lang):
                    available.add(code)
        return available

    def _model_files_present(self, code: str, lang: dict) -> bool:
        rec = lang.get("rec_model")
        if not rec:
            return False
        model_path = os.path.join(self.model_storage_dir, code, rec)
        if not os.path.isfile(model_path):
            return False
        # 字典文件若声明了也必须存在：v3.10.0 的 mobile rec 模型不内嵌字符表，
        # 缺失字典会在 CTCLabelDecode 抛 "character must not be None"，导致该语言
        # 静默识别失败却仍被判为「可用」。
        dict_name = lang.get("rec_dict")
        if dict_name:
            dict_path = os.path.join(self.model_storage_dir, code, dict_name)
            if not os.path.isfile(dict_path):
                return False
        return True

    def _rec_paths(self, code: str) -> tuple[str, str | None]:
        """返回 (rec_model_path, rec_dict_path)；无 rec_model 时返回 (None, None)。"""
        lang = self._catalog.get(code)
        if not lang or not lang.get("rec_model"):
            return None, None
        model_path = os.path.join(self.model_storage_dir, code, lang["rec_model"])
        dict_name = lang.get("rec_dict")
        dict_path = os.path.join(self.model_storage_dir, code, dict_name) if dict_name else None
        return model_path, dict_path

    def _group_key(self, code: str) -> str:
        """分组键：wheel 内置的 en/ch_sim 归入 'default'，其余语言各自成组。"""
        lang = self._catalog.get(code)
        if lang and lang.get("tier") == "builtin_wheel":
            return "default"
        return code

    # ------------------------------------------------------------------
    # 引擎构建
    # ------------------------------------------------------------------
    def _build_engine(self, rec_model_path: str | None = None, rec_keys_path: str | None = None):
        """
        构造引擎：**优先 DirectML，失败降级 CPU**。

        GPU 回退是口碑生死线——绝不能因为 provider 初始化失败就让整个 worker 闪退。
        降级 CPU 时仍保留 rec_model_path / rec_keys_path（多语言权重不受影响）。
        """
        from rapidocr_onnxruntime import RapidOCR

        kwargs: dict = {"det_use_dml": True, "cls_use_dml": True, "rec_use_dml": True}
        if rec_model_path:
            kwargs["rec_model_path"] = rec_model_path
        if rec_keys_path and os.path.isfile(rec_keys_path):
            kwargs["rec_keys_path"] = rec_keys_path

        try:
            # rapidocr_onnxruntime 的 kwargs 采用 det_/cls_/rec_ 前缀（1.4.x 签名），
            # 对应「det 检测 / cls 方向分类 / rec 识别」三段的 DML 开关。
            engine = RapidOCR(**kwargs)
            print("⚡ [OCR] DirectML enabled")
            return engine
        except Exception as e:
            # DML provider 不可用（标准 onnxruntime 而非 directml 包）时降级 CPU，
            # 绝不整体闪退——这是口碑生死线。
            print(f"⚠️ [OCR] DirectML init failed, falling back to CPU: {e}")
            cpu_kwargs = {k: v for k, v in kwargs.items() if not k.endswith("_use_dml")}
            return RapidOCR(**cpu_kwargs)

    def _get_engine_for_group(self, group_key: str, group_langs: list[str]):
        """获取（或懒加载）某个语言分组的引擎。"""
        if group_key not in self._engines:
            print(f"⏳ [OCR] Loading RapidOCR engine (group: {group_key}, langs: {group_langs}) ...")
            if group_key == "default":
                self._engines[group_key] = self._build_engine()
            else:
                model_path, dict_path = self._rec_paths(group_key)
                self._engines[group_key] = self._build_engine(model_path, dict_path)
        return self._engines[group_key]

    # ------------------------------------------------------------------
    # 抽象接口
    # ------------------------------------------------------------------
    def ocr_frame(self, img_rgb: np.ndarray, languages: list[str]) -> list[dict]:
        """
        对一帧图像执行 OCR，返回**行级**结构化结果（多语言合并）。

        Args:
            img_rgb: RGB 顺序的 numpy 数组（本项目内部统一用 RGB）
            languages: 语言列表，如 ['en', 'ch_sim', 'ja']

        Returns:
            [{
                "text": str,
                "bbox": (left, top, right, bottom)   # 归一化 0~1
                "score": float,
                "lang": str,        # 该行所属语言分组（置信度低于阈值时调用方可据此过滤）
                "corners": [[x, y], ...]  # 原始四角，供未来精确框选
            }]

        坐标语义：bbox 四值是该文本块**旋转外接矩形的直角包围盒**，
        归一化到 0~1，避免分辨率依赖；原始四角另行保留在 corners 里。
        """
        if not languages:
            languages = DEFAULT_OCR_LANGS

        # 过滤到目录里真正认识的语言；目录里没有的（未知码）直接丢弃
        langs = [l for l in languages if l in self._catalog]
        if not langs:
            langs = list(DEFAULT_OCR_LANGS)

        # 只跑有可用模型的语言；缺失的提示但不失败
        available = self.available_languages()
        run_langs = [l for l in langs if l in available]
        missing = [l for l in langs if l not in available]
        if missing:
            print(
                f"⚠️ [OCR] languages without an available rec model: {missing}. "
                f"Skipping them (download the model in Settings)."
            )
        if not run_langs:
            # 一个都跑不了：兜底回默认语言，仍不行就返回空
            run_langs = [l for l in DEFAULT_OCR_LANGS if l in available]
        if not run_langs:
            return []

        img_h, img_w = img_rgb.shape[:2]
        if img_h == 0 or img_w == 0:
            return []

        # RapidOCR 生态（cv2.imread）默认 BGR，故此处显式转换，
        # 避免 RGB/BGR 混淆导致识别率无端下降
        img_bgr = cv2.cvtColor(img_rgb, cv2.COLOR_RGB2BGR)

        # 按 rec 模型分组，逐组跑引擎并合并行级结果
        groups: dict[str, list[str]] = {}
        for l in run_langs:
            groups.setdefault(self._group_key(l), []).append(l)

        lines: list[dict] = []
        for group_key, group_langs in groups.items():
            try:
                engine = self._get_engine_for_group(group_key, group_langs)
                result, _elapse = engine(img_bgr)
            except Exception as e:
                print(f"⚠️ OCR extraction failed (group {group_key}): {e}")
                continue
            if not result:
                continue
            lang_tag = "_".join(sorted(group_langs))
            for box, text, score in result:
                if not text:
                    continue
                xs = [float(p[0]) for p in box]
                ys = [float(p[1]) for p in box]
                lines.append(
                    {
                        "text": str(text),
                        "bbox": (
                            _clamp01(min(xs) / img_w),
                            _clamp01(min(ys) / img_h),
                            _clamp01(max(xs) / img_w),
                            _clamp01(max(ys) / img_h),
                        ),
                        "score": float(score),
                        "lang": lang_tag,
                        # RapidOCR 返回的是按左上→右上→右下→左下的四角坐标
                        "corners": [[float(p[0]), float(p[1])] for p in box],
                    }
                )
        return lines

    def process_image(self, img_input: np.ndarray | str, languages: list[str]) -> str:
        """
        兼容旧调用：返回拼接全文。

        `ocr_text` 保留为拼接全文（兼容旧管道与旧搜索），
        但**检索与定位以行级 ocr_frame() 为真数据源**。
        """
        if isinstance(img_input, str):
            img_bgr = cv2.imread(img_input)
            if img_bgr is None:
                return ""
            img_input = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)

        lines = self.ocr_frame(img_input, languages)
        return " ".join(line["text"] for line in lines)


def _clamp01(v: float) -> float:
    """把归一化坐标夹到 [0, 1]——框可能略微超出画幅。"""
    if v < 0.0:
        return 0.0
    if v > 1.0:
        return 1.0
    return v


def lines_to_payload(corners: list[list[float]]) -> str:
    """把原始四角序列化为 OcrLine.payload_json。"""
    return json.dumps({"corners": corners}, ensure_ascii=False)
