"""
ZeroMQ 消息调度中心：接收 protobuf 请求，分发给各引擎处理。
"""
import os
import time
import traceback

import numpy as np
import zmq
import search_pb2
from PIL import Image

from config import PORT, DEFAULT_OCR_LANGS, MAX_FRAMES_PER_ENCODING_BATCH
from utils.logger import init_logger
from media import load_image, load_image_pil, extract_video_frames, is_video_file
from engines import OCRManager, SiglipEngine


class InferenceServer:
    """AI 推理服务：监听端口，分发 batch / ocr_task / file / text 请求。"""

    def __init__(self):
        # 1. 日志初始化
        log_path = init_logger()
        print(f"📝 Log file: {log_path}")

        # 2. 解析模型目录
        from config import get_model_dirs
        siglip_dir, ocr_dir = get_model_dirs()
        os.makedirs(ocr_dir, exist_ok=True)

        # 3. 初始化引擎
        print("⏳ Initializing OCR Engine...")
        self.ocr_manager: OCRManager = OCRManager(ocr_dir)

        print("⏳ Initializing SigLIP 2 Engine...")
        self.siglip: SiglipEngine = SiglipEngine(siglip_dir)

        # 4. 初始化 ZeroMQ REP 套接字
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.REP)
        self.socket.bind(f"tcp://127.0.0.1:{PORT}")
        print(f"🚀 AI Worker online! Listening on port {PORT}...")

    # ------------------------------------------------------------------
    # 请求处理方法
    # ------------------------------------------------------------------
    def _handle_batch(self, req: search_pb2.EncodeRequest) -> list[search_pb2.FrameResult]:
        """
        批量编码（含可选 OCR）。

        本方法同时处理图片与视频：
          - 图片：作为单帧 (timestamp=0.0) 处理；
          - 视频：调用 extract_video_frames 按 FPS 抽帧，每帧独立编码 + 可选 OCR，
            并携带各自相对视频起点的 timestamp 返回，使 Rust 侧能建立帧级索引。

        注意：此前实现把每个路径一律当图片 load_image_pil，视频会解码失败被静默跳过，
        导致扫描/索引阶段对视频返回 0 帧。此处已修复。
        """
        paths = req.batch.file_paths
        ocr_cfg = req.batch.ocr_config
        enable_ocr = ocr_cfg.enable_ocr if ocr_cfg else False
        # 注意：用 list() 拷贝，避免误改模块级默认列表
        ocr_langs = list(ocr_cfg.languages) if (ocr_cfg and ocr_cfg.languages) else list(DEFAULT_OCR_LANGS)

        print(f"[BATCH] {len(paths)} paths (OCR: {enable_ocr}, langs: {ocr_langs})")

        # 显式标注元素类型：否则空列表推断为 list[Unknown]，后续 append 会被判部分未知。
        result_frames: list[search_pb2.FrameResult] = []

        # 按路径逐个处理而非"先全部读入再统一编码"：
        # 一个 batch 里可能混入多个长视频，若一次性把全部帧收集到内存再编码，
        # 显存/内存峰值会被放大。逐路径处理可隔离单文件失败、限定内存占用。
        for p in paths:
            # 1) 统一构造本路径的帧任务列表：[(timestamp, PIL.Image)]
            frame_tasks: list[tuple[float, Image.Image]] = []
            try:
                if is_video_file(p):
                    # 视频抽帧：失败时 extract_video_frames 会抛错（文件不存在 / 无法解码），
                    # 由下方 except 兜底，跳过该文件而不影响同 batch 的其他文件
                    extracted = extract_video_frames(p)
                    for ts, frame_rgb in extracted:
                        frame_tasks.append((ts, Image.fromarray(frame_rgb)))
                    print(f"   [VIDEO] {p}: extracted {len(frame_tasks)} frames")
                else:
                    # 图片：单一帧，timestamp=0.0
                    pil_img = load_image_pil(p)
                    frame_tasks.append((0.0, pil_img))
            except Exception as e:
                # 单个文件读取/抽帧失败不应让整个 batch 崩溃
                print(f"⚠️ Failed to read {p}: {e}")
                continue

            if not frame_tasks:
                continue

            # 2) OCR（逐帧，避免一次性堆叠大数组导致 OOM）
            ocr_texts: list[str] = []
            for _, pil_img in frame_tasks:
                if enable_ocr:
                    ocr_texts.append(self.ocr_manager.process_image(np.array(pil_img), ocr_langs))
                else:
                    ocr_texts.append("")

            # 3) SigLIP 视觉编码：按子批次送入 ONNX，限定单次推理的帧数，
            #    防止长视频（上千帧）一次性推理撑爆显存。
            vectors: list[list[float]] = []
            for i in range(0, len(frame_tasks), MAX_FRAMES_PER_ENCODING_BATCH):
                chunk_imgs = [pil for (_, pil) in frame_tasks[i:i + MAX_FRAMES_PER_ENCODING_BATCH]]
                vectors.extend(self.siglip.embed_images(chunk_imgs))

            # 4) 组装 FrameResult。若编码返回的向量数与帧数不一致（理论上不应发生），
            #    按两者较小值裁剪，避免 zip/索引错位写入错乱向量。
            n = min(len(frame_tasks), len(vectors))
            for i in range(n):
                ts, _ = frame_tasks[i]
                result_frames.append(self._make_frame_result(
                    timestamp=ts, vector=vectors[i], ocr_text=ocr_texts[i], file_path=p
                ))

        return result_frames

    def _handle_ocr_task(self, req: search_pb2.EncodeRequest) -> list[search_pb2.FrameResult]:
        """按需 OCR 任务（只做文字识别，不编码向量）。"""
        paths = req.ocr_task.file_paths
        langs = list(req.ocr_task.languages) if req.ocr_task.languages else list(DEFAULT_OCR_LANGS)
        print(f"[OCR_TASK] {len(paths)} files, langs: {langs}")

        result_frames = []
        for p in paths:
            extracted = ""
            try:
                img_rgb = load_image(p)
                extracted = self.ocr_manager.process_image(img_rgb, langs)
            except Exception as e:
                print(f"⚠️ OCR failed for {p}: {e}")

            result_frames.append(self._make_frame_result(
                timestamp=0.0, vector=[], ocr_text=extracted, file_path=p
            ))
        return result_frames

    def _handle_file(self, req: search_pb2.EncodeRequest) -> list[search_pb2.FrameResult]:
        """单个文件（图片或视频）处理。"""
        file_path = req.file_path
        is_video = is_video_file(file_path)
        print(f"[FILE] Processing: {file_path} (video={is_video})")

        # OCR 配置：proto 的 single_file_ocr_config 若未设置，
        # 保持旧行为（始终 OCR、默认语言）以保证向后兼容
        if req.HasField("single_file_ocr_config"):
            ocr_cfg = req.single_file_ocr_config
            enable_ocr = ocr_cfg.enable_ocr
            ocr_langs = list(ocr_cfg.languages) if ocr_cfg.languages else list(DEFAULT_OCR_LANGS)
        else:
            enable_ocr = True
            ocr_langs = list(DEFAULT_OCR_LANGS)

        # 准备图像任务列表 (timestamp, rgb_array)
        image_tasks = []
        if is_video:
            image_tasks = extract_video_frames(file_path)
        else:
            try:
                img_rgb = load_image(file_path)
                image_tasks = [(0.0, img_rgb)]
            except Exception as e:
                print(f"⚠️ Failed to load {file_path}: {e}")

        result_frames = []
        for timestamp, frame_rgb in image_tasks:
            # OCR（可通过 single_file_ocr_config.enable_ocr 关闭）
            ocr_text = self.ocr_manager.process_image(frame_rgb, ocr_langs) if enable_ocr else ""

            # SigLIP 编码
            pil_img = Image.fromarray(frame_rgb)
            vector = self.siglip.embed_image(pil_img)

            if ocr_text:
                print(f"   [T={timestamp}s] OCR: {ocr_text[:80]}")

            result_frames.append(self._make_frame_result(
                timestamp=timestamp, vector=vector, ocr_text=ocr_text, file_path=file_path
            ))
        return result_frames

    def _handle_text(self, req: search_pb2.EncodeRequest) -> list[search_pb2.FrameResult]:
        """文本查询编码，支持 PING 心跳。"""
        if req.text == "PING_ENGINE":
            print("[PING] Health check OK")
            return []  # 返回空 frames 表示成功

        print(f"[TEXT] Query: {req.text}")
        vector = self.siglip.embed_text(req.text)
        return [self._make_frame_result(
            timestamp=0.0, vector=vector, ocr_text="", file_path=""
        )]

    # ------------------------------------------------------------------
    # 工具方法
    # ------------------------------------------------------------------
    def _make_frame_result(self, timestamp: float, vector: list[float], ocr_text: str, file_path: str) -> search_pb2.FrameResult:
        """构造一个 search_pb2.FrameResult。"""
        frame_res = search_pb2.FrameResult(
            timestamp=timestamp,
            index_time=time.time(),
            ocr_text=ocr_text,
            file_path=file_path
        )
        frame_res.vector.extend(vector)
        return frame_res

    def _send_error(self, task_id: str, message: str, code: int = 500) -> None:
        """发送错误响应。"""
        err = search_pb2.ErrorInfo(code=code, message=message, context="ai_inference")
        response = search_pb2.EncodeResponse(task_id=task_id, error=err)
        self.socket.send(response.SerializeToString())

    # ------------------------------------------------------------------
    # 主循环
    # ------------------------------------------------------------------
    def run(self):
        while True:
            # task_id 先置默认值：若 recv()/ParseFromString() 阶段就抛异常，
            # except 块里引用 task_id 也不会 NameError 导致整个 worker 崩溃。
            # 用空串而非 0：task_id 是 proto 的 string 字段，保持类型一致
            task_id = ""
            try:
                raw_req = self.socket.recv()
                req = search_pb2.EncodeRequest()
                req.ParseFromString(raw_req)
                task_id = req.task_id

                payload_type = req.WhichOneof("payload")
                print(f"📨 Request type: {payload_type}, task_id: {req.task_id}")

                # 路由分发
                if payload_type == "batch":
                    frames = self._handle_batch(req)
                elif payload_type == "ocr_task":
                    frames = self._handle_ocr_task(req)
                elif payload_type == "file_path":
                    frames = self._handle_file(req)
                elif payload_type == "text":
                    frames = self._handle_text(req)
                else:
                    print(f"⚠️ Unknown payload: {payload_type}")
                    self._send_error(req.task_id, "Unknown payload type", code=400)
                    continue

                # 发送成功响应
                success = search_pb2.SuccessPayload(frames=frames)
                response = search_pb2.EncodeResponse(task_id=req.task_id, success=success)
                self.socket.send(response.SerializeToString())

            except Exception as e:
                traceback.print_exc()
                print(f"❌ Fatal error: {e}")
                # REP 套接字在 recv 之后必须 reply，否则后续请求会全部卡死
                self._send_error(task_id, str(e), code=500)