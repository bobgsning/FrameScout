"""
图片读取工具：安全处理中文路径、BGR->RGB 转换、PIL 转换。
"""
import os
import cv2
import numpy as np
from PIL import Image


class ImageReadError(Exception):
    """图片读取失败时抛出。"""
    pass


def load_image(path: str) -> np.ndarray:
    """
    从磁盘读取一张图片（支持中文路径）。
    返回：RGB 格式的 numpy 数组 (H, W, 3)，uint8。
    失败时抛出 ImageReadError。
    """
    if not os.path.exists(path):
        raise ImageReadError(f"File not found: {path}")

    # ⚠️ 关键：用 np.fromfile 而不是 cv2.imread，避免中文路径问题
    data = np.fromfile(path, dtype=np.uint8)
    img_bgr = cv2.imdecode(data, cv2.IMREAD_COLOR)

    if img_bgr is None:
        raise ImageReadError(f"Failed to decode image: {path}")

    # BGR -> RGB
    img_rgb = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)
    return img_rgb


def load_image_rgb(path: str) -> np.ndarray:
    """load_image 的别名，语义更清晰。"""
    return load_image(path)


def load_image_pil(path: str) -> Image.Image:
    """
    读取图片并直接转为 PIL.Image（SigLIP processor 需要）。
    """
    img_rgb = load_image(path)
    return Image.fromarray(img_rgb)


def pil_from_rgb(img_rgb: np.ndarray) -> Image.Image:
    """从 RGB numpy 数组生成 PIL.Image。"""
    return Image.fromarray(img_rgb)