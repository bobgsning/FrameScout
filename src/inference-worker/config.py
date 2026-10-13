"""
全局配置模块：环境变量、端口、维度、路径解析。
"""
import io
import json
import os
import sys
import tempfile
import typing

# ⚠️ 离线模式：必须在任何 transformers 导入之前设置
os.environ["TRANSFORMERS_OFFLINE"] = "1"
os.environ["HF_HUB_OFFLINE"] = "1"

# 统一 stdout/stderr 为 UTF-8：中文 Windows 上，stdout 走管道/重定向时
# 默认编码是 GBK，代码中的 emoji（🚀🔥⚠️ 等）会直接触发 UnicodeEncodeError。
# （交互式控制台不受影响，这里是为被父进程启动等非交互场景兜底。）
for _stream in (sys.stdout, sys.stderr):
    # 运行时 sys.stdout 是 TextIOWrapper，但类型注解为 TextIO
    # （未声明 reconfigure）。用 cast 让静态检查识别，hasattr 做运行时防护
    stream = typing.cast(io.TextIOWrapper, _stream)
    if hasattr(stream, "reconfigure"):
        stream.reconfigure(encoding="utf-8", errors="replace")

# 服务端口
PORT = 16666

# 特征维度
EMBEDDING_DIM = 768

# 视频抽取参数
VIDEO_EXTRACT_FPS = 1
MAX_VIDEO_DURATION = 3600
# 单个视频最多抽取多少帧（无论 fps/duration 多大都不超过此值）。
# 默认 3600 = 1 FPS × 1 小时的自然上限，因此不会改变现有默认行为，
# 但在用户调高 VIDEO_EXTRACT_FPS 时可作为内存/耗时的硬性护栏。
MAX_FRAMES_PER_VIDEO = 3600

# 场景检测阈值（P2-3 / 第三轮 B1 / 债单 B9）。
# 帧间平均像素差（0~255）超过此值视为场景切换，额外保留关键帧。
# 文档建议场景分数 0.3（针对归一化 0~1）；这里用 0~255 的绝对差，
# 25 约对应归一化 0.1，对 PPT 切页/镜头切换敏感，对轻微抖动不敏感。
# 由 media/video_extractor.py 读取（债单 B9：此前定义后从不被读取，已修复）。
SCENE_CHANGE_THRESHOLD = 25.0

# 相邻去重阈值（P2-3 / 第三轮 B2）。
# 帧间平均像素差低于此值视为「几乎相同」，跳过（静止画面的冗余采样）。
SIMILAR_FRAME_SKIP_THRESHOLD = 2.5

# SigLIP 视觉编码的子批次大小。
# 一次 _handle_batch 可能包含多个视频、每个视频又有多帧，
# 若把全部帧一次性喂给 embed_images 会撑爆显存/内存峰值，
# 因此按此粒度分块送入 ONNX。16 在 DirectML/GPU 上为经验安全值。
MAX_FRAMES_PER_ENCODING_BATCH = 16

# 默认 OCR 语言
DEFAULT_OCR_LANGS = ["en"]

# ------------------------------------------------------------------
# BGE-M3 文本向量参数（B 线第 4 批 · dense-only）
# ------------------------------------------------------------------
# 文本通道是「第二条检索公路」：SigLIP 2 管画面（视觉），BGE-M3 管文字。
# 本轮只落 dense，sparse 暂缓（留给 FTS5 trigram 那批做精确匹配）。
BGE_MODEL_NAME = "bge-m3"
# BGE-M3 dense 维度（CLS token embedding 经 L2 归一化，已内置进 ONNX 导出）
BGE_DENSE_DIM = 1024
# 文本批量推理的子批次大小：长文本批量推理需限批，避免内存尖峰（文档 5.7）
BGE_MAX_BATCH = 8
# OCR 帧文本的 token 上限：一帧画面里的文字通常远小于此值，截断是安全的。
# （纯文本条目/长文档的切片走 chunk_index/parent_id，属后续 FTS5 批次）
BGE_MAX_TOKENS = 512


def get_base_path():
    """兼容 PyInstaller 打包和源码运行，返回可执行文件/主模块所在目录。"""
    if getattr(sys, "frozen", False):
        return os.path.dirname(sys.executable)
    return os.path.dirname(os.path.abspath(__file__))


def get_model_dirs():
    """
    返回 SigLIP2 与 OCR 模型目录。

    注意：PP-OCRv4 的默认权重随 `rapidocr_onnxruntime` 的 wheel 自带，无需下载。
    `ocr` 目录是留给将来按需下载的语言包 / 自训权重的占位。
    """
    base = get_base_path()
    siglip_dir = os.path.join(base, "models", "siglip2-base")
    ocr_dir = os.path.join(base, "models", "ocr")
    return siglip_dir, ocr_dir


def get_bge_dir():
    """返回 BGE-M3 模型目录（tokenizer 文件 + bge_m3_dense.onnx）。"""
    return os.path.join(get_base_path(), "models", "bge-m3")


# ------------------------------------------------------------------
# OCR 多语言语言目录（ocr_manifest.json）
# ------------------------------------------------------------------
# 单一事实源：语言码 / 分层（内置/可下载/规划）/ rec 模型与字典文件名 /
# 官方下载 URL 与 SHA256 全部集中在这份 manifest 里，由 worker、Rust 核心、
# 前端三处共同消费，避免多份硬编码漂移。
_ocr_manifest_cache: dict | None = None


def get_ocr_manifest_path() -> str:
    """返回 ocr_manifest.json 的绝对路径（worker 根目录，随打包复制）。"""
    return os.path.join(get_base_path(), "ocr_manifest.json")


def load_ocr_manifest(force_reload: bool = False) -> dict:
    """
    加载并解析 ocr_manifest.json，返回形如：
        {
          "schema_version": 1,
          "languages": [ {code, native, tier, deletable, rec_model?, rec_dict?, urls?, ...}, ... ]
        }
    结果带内存缓存；manifest 缺失时返回空的语言列表（不崩 worker，OCR 降级为
    默认中英权重，与「缺失模型不致命」的既有口径一致）。
    """
    global _ocr_manifest_cache
    if _ocr_manifest_cache is not None and not force_reload:
        return _ocr_manifest_cache

    manifest = {"schema_version": 1, "languages": []}
    try:
        with open(get_ocr_manifest_path(), "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, dict) and isinstance(data.get("languages"), list):
            manifest = data
    except FileNotFoundError:
        print("⚠️ [OCR] ocr_manifest.json not found — multilingual OCR disabled, default en+ch_sim only.")
    except (json.JSONDecodeError, OSError) as e:
        print(f"⚠️ [OCR] Failed to parse ocr_manifest.json: {e}")

    _ocr_manifest_cache = manifest
    return manifest


def get_log_dir():
    """返回并创建日志目录。"""
    log_dir = os.path.join(
        tempfile.gettempdir(),
        "FrameScout-Offline_AI_Search-Global",
        "logs"
    )
    os.makedirs(log_dir, exist_ok=True)
    return log_dir