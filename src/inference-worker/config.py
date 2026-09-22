"""
全局配置模块：环境变量、端口、维度、路径解析。
"""
import io
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
PORT = 5555

# 特征维度
EMBEDDING_DIM = 768

# 视频抽取参数
VIDEO_EXTRACT_FPS = 1
MAX_VIDEO_DURATION = 3600
# 单个视频最多抽取多少帧（无论 fps/duration 多大都不超过此值）。
# 默认 3600 = 1 FPS × 1 小时的自然上限，因此不会改变现有默认行为，
# 但在用户调高 VIDEO_EXTRACT_FPS 时可作为内存/耗时的硬性护栏。
MAX_FRAMES_PER_VIDEO = 3600

# SigLIP 视觉编码的子批次大小。
# 一次 _handle_batch 可能包含多个视频、每个视频又有多帧，
# 若把全部帧一次性喂给 embed_images 会撑爆显存/内存峰值，
# 因此按此粒度分块送入 ONNX。16 在 DirectML/GPU 上为经验安全值。
MAX_FRAMES_PER_ENCODING_BATCH = 16

# 默认 OCR 语言
DEFAULT_OCR_LANGS = ["en"]


def get_base_path():
    """兼容 PyInstaller 打包和源码运行，返回可执行文件/主模块所在目录。"""
    if getattr(sys, "frozen", False):
        return os.path.dirname(sys.executable)
    return os.path.dirname(os.path.abspath(__file__))


def get_model_dirs():
    """返回 SigLIP2 与 EasyOCR 模型目录。"""
    base = get_base_path()
    siglip_dir = os.path.join(base, "models", "siglip2-base")
    ocr_dir = os.path.join(base, "models", "easyocr")
    return siglip_dir, ocr_dir


def get_log_dir():
    """返回并创建日志目录。"""
    log_dir = os.path.join(
        tempfile.gettempdir(),
        "FrameScout-Offline_AI_Search-Global",
        "logs"
    )
    os.makedirs(log_dir, exist_ok=True)
    return log_dir