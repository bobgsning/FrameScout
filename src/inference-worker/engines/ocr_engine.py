"""
OCR 引擎：RapidOCR 懒加载 + 行级结果输出。

============================================================
 概念澄清（最容易糊的一点）
============================================================
  · PP-OCRv4 = 百度的**模型体系**（det / cls / rec 三段权重）
  · ONNX 版  = paddle2onnx 导出，脱离 PaddlePaddle 大框架
  · RapidOCR = **推理封装引擎**（不是模型本身），默认即加载 PP-OCRv4 的 ONNX 权重

 结论：不是二选一，而是「RapidOCR 的执行管线 + PP-OCRv4 的 ONNX 权重」。

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
from config import DEFAULT_OCR_LANGS

# 引擎标识随 FrameResult 一起落库：将来换引擎时，能据此区分「这批文本是谁认的」，
# 便于对比与回退。换引擎只需改这一处常量。
OCR_ENGINE_NAME = "rapidocr"
# 模型标识：记录权重体系，便于日后按模型维度评估识别质量
OCR_MODEL_NAME = "ppocrv4"

# PP-OCRv4 的默认权重覆盖「中文简体 + 英文」。
# 其余语言（日文 / 韩文 / 繁体等）需要按需下载语言包——链条短板在 OCR 侧：
# OCR 认不出的语言，文本模型再强也拿不到内容。
# 命中不支持的语言时**只提示、不失败**：系统有感知，但不替用户中断流程。
SUPPORTED_LANGUAGES = {"en", "ch_sim"}


class OCRManager:
    """
    RapidOCR 引擎管理。

    按语言组合懒加载并缓存引擎，避免为不使用的语言预分配显存/内存。
    注意：语言支持由权决定而非由引擎决定，因此非默认语言只能提示后按默认权重执行。
    """

    def __init__(self, model_storage_dir: str):
        """
        Args:
            model_storage_dir: 预留的自定义权重目录。
                PP-OCRv4 默认权重随 wheel 自带，无需下载；此目录留给将来按需
                下载的语言包或自训权重。
        """
        self.model_storage_dir = model_storage_dir
        self._engines: dict[str, object] = {}

    # ------------------------------------------------------------------
    # 引擎获取
    # ------------------------------------------------------------------
    def get_engine(self, languages: list[str]):
        """获取（或懒加载）指定语言的 RapidOCR 引擎。"""
        if not languages:
            languages = DEFAULT_OCR_LANGS

        unsupported = [l for l in languages if l not in SUPPORTED_LANGUAGES]
        if unsupported:
            print(
                f"⚠️ [OCR] languages not covered by the bundled PP-OCRv4 weights: {unsupported}. "
                f"Falling back to the default weights (en + ch_sim)."
            )

        cache_key = "_".join(sorted(languages))
        if cache_key not in self._engines:
            print(f"⏳ [OCR] Loading RapidOCR engine (langs: {languages}) ...")
            self._engines[cache_key] = self._build_engine()
        return self._engines[cache_key]

    def _build_engine(self):
        """
        构造引擎：**优先 DirectML，失败降级 CPU**。

        GPU 回退是口碑生死线——绝不能因为 provider 初始化失败就让整个 worker 闪退。
        """
        from rapidocr_onnxruntime import RapidOCR

        try:
            # rapidocr_onnxruntime 的 kwargs 采用 det_/cls_/rec_ 前缀（1.4.x 签名），
            # 对应「det 检测 / cls 方向分类 / rec 识别」三段的 DML 开关。
            engine = RapidOCR(det_use_dml=True, cls_use_dml=True, rec_use_dml=True)
            print("⚡ [OCR] DirectML enabled")
            return engine
        except Exception as e:
            # DML provider 不可用（标准 onnxruntime 而非 directml 包）时降级 CPU，
            # 绝不整体闪退——这是口碑生死线。
            print(f"⚠️ [OCR] DirectML init failed, falling back to CPU: {e}")
            return RapidOCR()

    # ------------------------------------------------------------------
    # 抽象接口
    # ------------------------------------------------------------------
    def ocr_frame(self, img_rgb: np.ndarray, languages: list[str]) -> list[dict]:
        """
        对一帧图像执行 OCR，返回**行级**结构化结果。

        Args:
            img_rgb: RGB 顺序的 numpy 数组（本项目内部统一用 RGB）
            languages: 语言列表，如 ['en', 'ch_sim']

        Returns:
            [{
                "text": str,
                "bbox": (left, top, right, bottom)   # 归一化 0~1
                "score": float,
                "lang": str,        # 语言标签；置信度低于阈值时调用方可据此过滤
                "corners": [[x, y], ...]  # 原始四角，供未来精确框选
            }]

        坐标语义：bbox 四值是该文本块**旋转外接矩形的直角包围盒**，
        归一化到 0~1，避免分辨率依赖；原始四角另行保留在 corners 里。
        """
        try:
            engine = self.get_engine(languages)
            # RapidOCR 生态（cv2.imread）默认 BGR，故此处显式转换，
            # 避免 RGB/BGR 混淆导致识别率无端下降
            img_bgr = cv2.cvtColor(img_rgb, cv2.COLOR_RGB2BGR)
            result, _elapse = engine(img_bgr)
        except Exception as e:
            print(f"⚠️ OCR extraction failed: {e}")
            return []

        if not result:
            return []

        img_h, img_w = img_rgb.shape[:2]
        if img_h == 0 or img_w == 0:
            return []

        lang_tag = "_".join(sorted(languages)) if languages else "en"
        lines: list[dict] = []
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
