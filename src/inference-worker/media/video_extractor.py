"""
视频抽帧工具：基于场景检测的关键帧抽取 + 相邻去重 + 生成器流式输出。

v3.2.0 第三轮 P0-6 / P2-3 改造：
  - 旧实现：固定 1 FPS 均匀采样 + `return list`，长视频 3600 帧一次性堆进内存
    （1080p × 3600 ≈ 22GB），server.py 再把全部帧堆进 frame_tasks ⇒ OOM。
  - 新实现：1 FPS 兜底采样 + 帧差分场景检测（场景变化 > 阈值时额外保留关键帧）
    + 相邻相似帧去重（> 0.98 跳过） + 生成器 yield（边抽边出，内存恒定）。
  - 兑现旧注释 `# TODO v3.2: 替换为基于场景检测(scene-detection)的抽帧`。

债单 B9 修复：旧实现 SCENE_CHANGE_THRESHOLD 定义后从不读取，抽帧只是「固定 1 FPS +
相邻去重」。现真正读取阈值做场景检测——以 0.25s 探测间隔采样，帧间差超过
SCENE_CHANGE_THRESHOLD 即判定场景切换并保留关键帧，其余按 1 FPS 兜底 + 相邻去重。
"""
import os
import shutil
import subprocess
import tempfile
from itertools import chain

import cv2
import numpy as np

from config import (
    VIDEO_EXTRACT_FPS,
    MAX_VIDEO_DURATION,
    MAX_FRAMES_PER_VIDEO,
    SCENE_CHANGE_THRESHOLD,
    SIMILAR_FRAME_SKIP_THRESHOLD,
    get_base_path,
)


# 常见视频扩展名白名单。
# 注意：Rust 侧 src-tauri/src/constants.rs 的 VIDEO_EXTENSIONS 必须与此保持一致，
# 否则会出现"扫描能枚举到、但 Python 当图片解码失败被跳过"的静默丢帧问题。
VIDEO_EXTENSIONS = {".mp4", ".mov", ".avi", ".mkv", ".webm", ".flv", ".m4v", ".ts"}
FFMPEG_FRAME_SIZE = 256


def _frame_mean_diff(a: np.ndarray, b: np.ndarray) -> float:
    """计算两帧的平均绝对像素差（0~255）。尺寸不一致时先 resize 对齐。"""
    if a.shape != b.shape:
        h = min(a.shape[0], b.shape[0], 256)
        w = min(a.shape[1], b.shape[1], 256)
        a = cv2.resize(a, (w, h))
        b = cv2.resize(b, (w, h))
    # 灰度比较更稳：颜色噪声（JPEG 压缩、亮度抖动）不会误判为场景变化
    gray_a = cv2.cvtColor(a, cv2.COLOR_RGB2GRAY) if a.ndim == 3 else a
    gray_b = cv2.cvtColor(b, cv2.COLOR_RGB2GRAY) if b.ndim == 3 else b
    return float(np.mean(np.abs(gray_a.astype(np.float32) - gray_b.astype(np.float32))))


def _resolve_ffmpeg() -> str:
    """Find the packaged FFmpeg first, then fall back to PATH for development."""
    packaged = os.path.join(get_base_path(), "ffmpeg.exe")
    if os.path.isfile(packaged):
        return packaged
    system = shutil.which("ffmpeg")
    if system:
        return system
    raise FileNotFoundError(
        "FFmpeg fallback is unavailable: ffmpeg.exe was not packaged and ffmpeg is not on PATH"
    )


def _iter_opencv_frames(video_path: str, probe_fps: float, max_duration: float):
    """Yield regularly sampled RGB frames through OpenCV."""
    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        cap.release()
        raise ValueError("OpenCV could not open the video")

    try:
        fps = cap.get(cv2.CAP_PROP_FPS)
        if not fps or fps != fps:
            fps = 25.0

        sec = 0.0
        yielded = 0
        step = 1.0 / probe_fps
        while sec <= max_duration:
            cap.set(cv2.CAP_PROP_POS_FRAMES, int(fps * sec))
            ok, frame = cap.read()
            if not ok:
                if yielded == 0:
                    raise ValueError("OpenCV could not decode the first frame")
                break
            yielded += 1
            yield sec, cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            sec += step
    finally:
        cap.release()


def _iter_ffmpeg_frames(video_path: str, probe_fps: float, max_duration: float):
    """Yield fixed-size RGB frames from one FFmpeg process."""
    size = FFMPEG_FRAME_SIZE
    frame_bytes = size * size * 3
    vf = (
        f"fps={probe_fps},"
        f"scale={size}:{size}:force_original_aspect_ratio=decrease,"
        f"pad={size}:{size}:(ow-iw)/2:(oh-ih)/2:black"
    )
    command = [
        _resolve_ffmpeg(),
        "-nostdin",
        "-v",
        "error",
        "-i",
        video_path,
        "-t",
        str(max_duration),
        "-an",
        "-sn",
        "-dn",
        "-vf",
        vf,
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "pipe:1",
    ]
    creation_flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    with tempfile.TemporaryFile() as error_file:
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=error_file,
            creationflags=creation_flags,
        )
        if process.stdout is None:
            process.kill()
            raise ValueError("FFmpeg stdout pipe was not created")

        yielded = 0
        try:
            while True:
                data = bytearray()
                while len(data) < frame_bytes:
                    chunk = process.stdout.read(frame_bytes - len(data))
                    if not chunk:
                        break
                    data.extend(chunk)
                if not data:
                    break
                if len(data) != frame_bytes:
                    raise ValueError("FFmpeg returned a truncated video frame")
                frame = np.frombuffer(data, dtype=np.uint8).reshape(size, size, 3).copy()
                yield yielded / probe_fps, frame
                yielded += 1

            return_code = process.wait()
            error_file.seek(0)
            error = error_file.read().decode("utf-8", errors="replace").strip()
            if yielded == 0 or return_code != 0:
                raise ValueError(
                    f"FFmpeg could not decode video: {error or 'no frames returned'}"
                )
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            process.stdout.close()


def extract_video_frames(
    video_path: str,
    extract_fps: float = VIDEO_EXTRACT_FPS,
    max_duration: float = MAX_VIDEO_DURATION,
    max_frames: int = MAX_FRAMES_PER_VIDEO,
):
    """
    从视频中抽取关键帧，**生成器 yield**（不再返回 list）。

    抽帧策略（三道护栏 + 场景检测 + 去重）：
      1. 1 FPS 兜底采样：保证静止画面也能被采样到（旧默认行为）；
      2. 场景检测：帧间差 > SCENE_CHANGE_THRESHOLD 时额外保留该帧
         （PPT 切页 / 镜头切换 / 高速动作段落不再漏采）；
      3. 相邻去重：帧间差 < SIMILAR_FRAME_SKIP_THRESHOLD 时跳过
         （静止画面不再每秒写一个几乎相同的向量）；
      4. 三道护栏（max_duration / max_frames / 视频结束）任一触发即停止。

    生成器语义：调用方边迭代边消费，内存占用恒定（不再一次性堆 22GB）。
    调用方需在迭代结束后不再使用生成器；若提前 break，VideoCapture 由 GC 释放。

    yield: (timestamp: float, frame_rgb: np.ndarray)
    """
    if not os.path.exists(video_path):
        raise FileNotFoundError(f"Video not found: {video_path}")

    base_step = 1.0 / extract_fps
    probe_fps = extract_fps * 4.0
    try:
        opencv_frames = _iter_opencv_frames(video_path, probe_fps, max_duration)
        first = next(opencv_frames)
        raw_frames = chain([first], opencv_frames)
    except (StopIteration, ValueError) as opencv_error:
        print(f"⚠️ OpenCV decode failed, using FFmpeg fallback: {opencv_error}")
        raw_frames = _iter_ffmpeg_frames(video_path, probe_fps, max_duration)

    yielded = 0
    prev_frame = None
    last_yield_sec = -base_step

    for sec, frame_rgb in raw_frames:
        if prev_frame is None:
            yield sec, frame_rgb
            yielded += 1
            prev_frame = frame_rgb
            last_yield_sec = sec
            if yielded >= max_frames:
                break
            continue

        diff = _frame_mean_diff(prev_frame, frame_rgb)
        if diff >= SCENE_CHANGE_THRESHOLD:
            yield sec, frame_rgb
            yielded += 1
            prev_frame = frame_rgb
            last_yield_sec = sec
            if yielded >= max_frames:
                break
            continue

        if diff < SIMILAR_FRAME_SKIP_THRESHOLD:
            continue

        if sec - last_yield_sec >= base_step:
            yield sec, frame_rgb
            yielded += 1
            prev_frame = frame_rgb
            last_yield_sec = sec
            if yielded >= max_frames:
                break


def extract_video_frames_list(
    video_path: str,
    extract_fps: float = VIDEO_EXTRACT_FPS,
    max_duration: float = MAX_VIDEO_DURATION,
    max_frames: int = MAX_FRAMES_PER_VIDEO,
) -> list[tuple[float, np.ndarray]]:
    """
    把生成器收集成 list 的便捷封装（向后兼容旧调用方）。

    ⚠️ 这会一次性把全部帧堆进内存，长视频会 OOM。
    新代码应直接迭代 `extract_video_frames`，按子批次处理。
    保留此函数仅为兼容 `_handle_file`（单文件路径，帧数通常不多）。
    """
    return list(extract_video_frames(video_path, extract_fps, max_duration, max_frames))


def is_video_file(path: str) -> bool:
    """根据扩展名白名单判断给定路径是否为视频文件。"""
    ext = os.path.splitext(path)[-1].lower()
    return ext in VIDEO_EXTENSIONS
