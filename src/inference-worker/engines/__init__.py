"""
AI 引擎包：OCR、SigLIP 视觉与 BGE-M3 文本特征提取。
"""
from .ocr_engine import (
    OCR_ENGINE_NAME,
    OCR_MODEL_NAME,
    OCRManager,
    lines_to_payload,
)
from .siglip_engine import SiglipEngine
from .bge_engine import BgeEngine

__all__ = [
    "OCRManager",
    "OCR_ENGINE_NAME",
    "OCR_MODEL_NAME",
    "lines_to_payload",
    "SiglipEngine",
    "BgeEngine",
]
