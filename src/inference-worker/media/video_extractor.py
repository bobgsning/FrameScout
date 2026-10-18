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
import threading
import time
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

# FFmpeg 兜底解码参数（PR #6 真金 · 本地采纳）。
# FFmpeg 路径固定输出 256×256（OpenCV 路径为原始分辨率）：SigLIP 本身按 256 输入，
# 但两路径喂给 _frame_mean_diff 的绝对像素差量纲不同，场景检测阈值会略有差异，
# 已在模块 docstring 与复盘文档中注明，属可接受的工程取舍。
FFMPEG_FRAME_SIZE = 256
# 单帧读取超时（秒）：损坏/卡死的视频流不能让 worker 单线程 REQ/REP 循环永远阻塞。
# watchdog 线程在超过此窗口仍未读满一帧时 kill 掉 ffmpeg 进程，确保 finally 清理必执行。
FFMPEG_READ_TIMEOUT = 30.0


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
    """定位 ffmpeg 可执行文件：优先打包在 worker 同级的 ffmpeg.exe，其次 PATH（开发期）。

    与 Rust 侧 `ffmpeg_cmd.rs::resolve_ffmpeg_path` 保持同一语义：打包发布包把
    ffmpeg.exe 放在 ai_worker.exe 同级（config.get_base_path()），源码运行时回退 PATH。
    """
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
    """通过 OpenCV 以 probe_fps 密度顺序抽帧（原始分辨率），yield (sec, frame_rgb)。

    打开失败或首帧解码失败时抛 ValueError，由调用方触发 FFmpeg 兜底。
    """
    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        cap.release()
        raise ValueError(f"OpenCV could not open the video: {video_path}")
    try:
        # 某些容器/编码会读到 fps=0 或 NaN，此时按 25fps 兜底
        fps = cap.get(cv2.CAP_PROP_FPS)
        if not fps or fps != fps:  # 处理 NaN / 0
            fps = 25.0
        step = 1.0 / probe_fps
        sec = 0.0
        yielded = 0
        while sec <= max_duration:
            cap.set(cv2.CAP_PROP_POS_FRAMES, int(fps * sec))
            ok, frame = cap.read()
            if not ok:
                if yielded == 0:
                    raise ValueError(f"OpenCV could not decode the first frame: {video_path}")
                break
            yielded += 1
            yield sec, cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            sec += step
    finally:
        cap.release()


def _iter_ffmpeg_frames(video_path: str, probe_fps: float, max_duration: float):
    """通过外部 ffmpeg 进程以 probe_fps 密度抽帧（固定 256×256 RGB），yield (sec, frame_rgb)。

    三处工程细节（来自 PR #6，本地采纳）：
      - stderr 写临时文件（非 PIPE），避免 stdout/stderr 管道缓冲死锁；
      - np.frombuffer(...).copy()，保证帧数据生命周期安全；
      - 单进程解码、不每帧 spawn；-an/-sn/-dn 丢弃音轨/字幕/数据流。
    额外补 PR 没做好的读超时：watchdog 线程在 FFMPEG_READ_TIMEOUT 内读不满一帧时
    kill 进程，确保 finally 的清理必执行、不卡死 worker 单线程 REQ/REP 循环。
    """
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
        "-v", "error",
        "-i", video_path,
        "-t", str(max_duration),
        "-an", "-sn", "-dn",
        "-vf", vf,
        "-f", "rawvideo",
        "-pix_fmt", "rgb24",
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

        # watchdog：每读满一帧刷新时间戳，超过窗口仍未读到新帧则 kill 进程，
        # 让阻塞中的 read() 立即返回 EOF，主循环得以进入 finally 清理。
        last_read = [time.monotonic()]
        stop_event = threading.Event()

        def _watchdog() -> None:
            while not stop_event.is_set():
                time.sleep(1.0)
                if stop_event.is_set():
                    return
                if time.monotonic() - last_read[0] > FFMPEG_READ_TIMEOUT:
                    try:
                        if process.poll() is None:
                            process.kill()
                    except OSError:
                        pass
                    return

        watcher = threading.Thread(target=_watchdog, daemon=True)
        watcher.start()

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
                last_read[0] = time.monotonic()
                frame = np.frombuffer(data, dtype=np.uint8).reshape(size, size, 3).copy()
                yield yielded / probe_fps, frame
                yielded += 1

            return_code = process.wait()
            error_file.seek(0)
            error = error_file.read().decode("utf-8", errors="replace").strip()
            # 首帧都没抽出来：硬失败（视频完全解不了），必须 raise。
            if yielded == 0:
                raise ValueError(
                    f"FFmpeg could not decode video: {error or 'no frames returned'}"
                )
            # 已产出部分帧后异常退出（watchdog 杀进程 / 流中途损坏）：保留已抽到的帧，
            # 只打印可见警告，不整体判失败——视频前半段可解码、有价值，不应作废。
            if return_code != 0:
                print(
                    f"⚠️ FFmpeg decode interrupted for {os.path.basename(video_path)} "
                    f"after {yielded} frames: {error or f'exit code {return_code}'}"
                )
        finally:
            stop_event.set()
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

    帧来源二选一：
      - 主路径：OpenCV 按原始分辨率抽帧；
      - 兜底：OpenCV 打不开容器/编解码器或首帧解码失败时，改由 FFmpeg 进程按
        固定 256×256 抽帧（MKV/AVI 等 OpenCV 解不了的 codec 不再静默丢成 0 帧）。
    两条路径最终都汇入下面同一套「场景检测 + 相邻去重 + 三道护栏」选择逻辑。

    ⚠️ 分辨率口径差异：FFmpeg 路径固定 256×256，OpenCV 路径为原始分辨率，
    `_frame_mean_diff` 的绝对像素差量纲不同，故两路径的场景检测阈值存在轻微差异。
    属已知取舍（SigLIP 本身按 256 输入，检索不受影响），详见复盘文档。

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

    base_step = 1.0 / extract_fps          # 兜底采样间隔（默认 1s）
    probe_fps = extract_fps * 4.0          # 场景检测探测密度（默认 0.25s 一帧，4x）

    # 帧来源二选一：OpenCV 主路径（原始分辨率），失败则 FFmpeg 兜底（256×256）。
    # 先取第一帧以触发「打不开/首帧解码失败」的异常判定，再连回迭代器继续消费。
    try:
        opencv_frames = _iter_opencv_frames(video_path, probe_fps, max_duration)
        first = next(opencv_frames)
        raw_frames = chain([first], opencv_frames)
    except (StopIteration, ValueError) as opencv_error:
        print(f"⚠️ OpenCV decode failed, using FFmpeg fallback: {opencv_error}")
        raw_frames = _iter_ffmpeg_frames(video_path, probe_fps, max_duration)

    yielded = 0  # 抽到的帧数（try 外定义，供 finally 打印截断日志）
    prev_frame = None  # 上一次实际 yield 出去的帧，用于相邻去重与场景检测
    last_yield_sec = -base_step  # 上次兜底采样的时间戳

    try:
        for sec, frame_rgb in raw_frames:
            if prev_frame is None:
                # 第一帧无条件保留
                yield (sec, frame_rgb)
                yielded += 1
                prev_frame = frame_rgb
                last_yield_sec = sec
                if yielded >= max_frames:
                    break
                continue

            diff = _frame_mean_diff(prev_frame, frame_rgb)

            # 债单 B9：场景检测真正生效——帧间差超过 SCENE_CHANGE_THRESHOLD 视为
            # 场景切换（PPT 切页 / 镜头切换），立即保留该关键帧。
            if diff >= SCENE_CHANGE_THRESHOLD:
                yield (sec, frame_rgb)
                yielded += 1
                prev_frame = frame_rgb
                last_yield_sec = sec
                if yielded >= max_frames:
                    break
                continue

            # 相邻去重：与上次保留的帧几乎相同，跳过（静止画面的冗余采样）
            if diff < SIMILAR_FRAME_SKIP_THRESHOLD:
                continue

            # 中间地带（轻微变化）：按 base_step 兜底采样，避免漏采
            if sec - last_yield_sec >= base_step:
                yield (sec, frame_rgb)
                yielded += 1
                prev_frame = frame_rgb
                last_yield_sec = sec
                if yielded >= max_frames:
                    break
    finally:
        # M2'''' 缓冲（视频抽帧参数可调延期）：达到 max_frames 上限时打印可见日志，
        # 告知「只索引了前 N 帧」，避免用户误以为整段视频都入库。
        if yielded >= max_frames:
            print(
                f"[video] {os.path.basename(video_path)}: indexed {yielded} frames "
                f"(hit MAX_FRAMES_PER_VIDEO={max_frames}); later frames were NOT indexed"
            )


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
