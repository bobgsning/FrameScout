"""
视频抽帧工具：按指定 FPS 均匀抽取帧。
"""
import os
import cv2
import numpy as np

from config import VIDEO_EXTRACT_FPS, MAX_VIDEO_DURATION, MAX_FRAMES_PER_VIDEO


# 常见视频扩展名白名单。
# 注意：Rust 侧 src-tauri/src/constants.rs 的 VIDEO_EXTENSIONS 必须与此保持一致，
# 否则会出现"扫描能枚举到、但 Python 当图片解码失败被跳过"的静默丢帧问题。
VIDEO_EXTENSIONS = {".mp4", ".mov", ".avi", ".mkv", ".webm", ".flv", ".m4v", ".ts"}


def extract_video_frames(
    video_path: str,
    extract_fps: float = VIDEO_EXTRACT_FPS,
    max_duration: float = MAX_VIDEO_DURATION,
    max_frames: int = MAX_FRAMES_PER_VIDEO,
) -> list[tuple[float, np.ndarray]]:
    """
    从视频中按指定 FPS 抽取帧。

    三道护栏，任意一道触发即停止抽帧：
      - max_duration : 抽到该秒数（相对视频起点）为止，避免对超长视频无脑抽到底；
      - max_frames   : 累计帧数硬上限，当用户调高 extract_fps 时仍能限定耗时与内存;
      - 视频结束     : cap.read() 返回 False 即自然终止。

    默认：1 FPS、最长 1 小时、最多 3600 帧。
    TODO v3.2: 替换为基于场景检测(scene-detection)的抽帧。

    返回：List of (timestamp: float, frame_rgb: np.ndarray)
    """
    if not os.path.exists(video_path):
        raise FileNotFoundError(f"Video not found: {video_path}")

    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        # 文件存在但 OpenCV 无法解码（损坏 / 编码不支持），
        # 显式报错让上层返回 500，而不是静默返回 0 帧
        raise ValueError(f"Cannot open video (corrupted or unsupported codec): {video_path}")

    # 某些容器/编码会读到 fps=0 或 NaN，此时按 25fps 兜底，
    # 保证抽帧步进计算不至于除零或死循环
    fps = cap.get(cv2.CAP_PROP_FPS)
    if not fps or fps != fps:  # 处理 NaN / 0
        fps = 25.0

    frames = []
    sec = 0.0
    step = 1.0 / extract_fps

    # 注意：这里是"逐帧 seek + read"而非顺序解码。
    # 优点：内存占用恒定；缺点：seek 在长视频上较慢。
    while True:
        frame_id = int(fps * sec)
        cap.set(cv2.CAP_PROP_POS_FRAMES, frame_id)
        ret, frame = cap.read()
        if not ret:
            break

        # OpenCV 内部为 BGR，SigLIP / EasyOCR 均期望 RGB
        frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        frames.append((sec, frame_rgb))

        # 帧数硬上限：先于 duration 判断，确保无论 fps 多大都不会失控
        if len(frames) >= max_frames:
            break

        sec += step
        if sec > max_duration:
            break

    cap.release()
    return frames


def is_video_file(path: str) -> bool:
    """根据扩展名白名单判断给定路径是否为视频文件。"""
    ext = os.path.splitext(path)[-1].lower()
    return ext in VIDEO_EXTENSIONS