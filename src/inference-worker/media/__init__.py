"""
多媒体处理包：图片读取、视频抽帧。
"""
from .image_io import load_image, load_image_rgb, load_image_pil, pil_from_rgb
from .video_extractor import extract_video_frames, is_video_file

__all__ = [
    "load_image",
    "load_image_rgb",
    "load_image_pil",
    "pil_from_rgb",
    "extract_video_frames",
    "is_video_file",
]
