"""
ZeroMQ 消息调度中心：接收 protobuf 请求，分发给各引擎处理。
"""
import os
import time
import traceback

import numpy as np
import zmq
import framescout_pb2
from PIL import Image

from config import PORT, DEFAULT_OCR_LANGS, MAX_FRAMES_PER_ENCODING_BATCH
from engines import OCR_ENGINE_NAME, OCR_MODEL_NAME, lines_to_payload
from utils.logger import init_logger
from media import load_image, load_image_pil, extract_video_frames, extract_video_frames_list, is_video_file
from engines import OCRManager, SiglipEngine, BgeEngine


class InferenceServer:
    """AI 推理服务：监听端口，分发 batch / ocr_task / file / text 请求。"""

    def __init__(self):
        # 1. 日志初始化
        log_path = init_logger()
        print(f"📝 Log file: {log_path}")

        # 2. 解析模型目录
        from config import get_model_dirs, get_bge_dir
        siglip_dir, ocr_dir = get_model_dirs()
        bge_dir = get_bge_dir()
        os.makedirs(ocr_dir, exist_ok=True)
        os.makedirs(bge_dir, exist_ok=True)

        # 3. 初始化引擎
        print("⏳ Initializing OCR Engine...")
        self.ocr_manager: OCRManager = OCRManager(ocr_dir)

        print("⏳ Initializing SigLIP 2 Engine...")
        self.siglip: SiglipEngine = SiglipEngine(siglip_dir)

        # BGE-M3 是增量能力：模型缺省时 available=False，不影响视觉主链路。
        print("⏳ Initializing BGE-M3 Text Engine (optional)...")
        self.bge: BgeEngine = BgeEngine(bge_dir)

        # 4. 初始化 ZeroMQ REP 套接字
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.REP)
        self.socket.bind(f"tcp://127.0.0.1:{PORT}")
        print(f"🚀 AI Worker online! Listening on port {PORT}...")

    # ------------------------------------------------------------------
    # 请求处理方法
    # ------------------------------------------------------------------
    def _process_frame_sub_batch(
        self,
        file_path: str,
        frames: list[tuple[float, np.ndarray]],
        enable_ocr: bool,
        ocr_langs: list[str],
        result_frames: list,
    ) -> None:
        """
        处理一个文件的子批次帧：OCR + BGE 文本向量 + SigLIP 视觉向量 + 组装结果。

        抽出此方法（P0-6 / 第三轮 B4）：
          - 旧实现把视频全部帧一次性堆进 frame_tasks（1080p×3600 ≈ 22GB）⇒ OOM；
          - 新实现按 MAX_FRAMES_PER_ENCODING_BATCH 一批，边抽帧边编码边组装，
            内存占用恒定（一批 ≈ 16 帧 × 6.2MB ≈ 100MB）。

        frames: [(timestamp, frame_rgb)]，本方法消费完后即可释放。
        result_frames: 把组装好的 FrameResult append 进此列表。
        """
        if not frames:
            return

        # 1) OCR（逐帧）
        ocr_texts: list[str] = []
        ocr_lines_per_frame: list[list[dict]] = []
        for _, frame_rgb in frames:
            if enable_ocr:
                lines = self.ocr_manager.ocr_frame(frame_rgb, ocr_langs)
                ocr_lines_per_frame.append(lines)
                ocr_texts.append(" ".join(line["text"] for line in lines))
            else:
                ocr_lines_per_frame.append([])
                ocr_texts.append("")

        # 2) BGE-M3 文本向量（dense-only）
        dense_per_frame = self.bge.embed_texts(ocr_texts)

        # 3) SigLIP 视觉编码：本子批次已限定帧数，无需再切分
        pil_imgs = [Image.fromarray(fr) for (_, fr) in frames]
        vectors = self.siglip.embed_images(pil_imgs)

        # 4) 组装 FrameResult
        n = min(len(frames), len(vectors))
        for i in range(n):
            ts, _ = frames[i]
            result_frames.append(self._make_frame_result(
                timestamp=ts, vector=vectors[i], ocr_text=ocr_texts[i], file_path=file_path,
                ocr_lines=ocr_lines_per_frame[i],
                frame_timestamp_ms=int(ts * 1000),
                dense_vector=dense_per_frame[i],
            ))

    def _handle_batch(self, req: framescout_pb2.EncodeRequest) -> list[framescout_pb2.FrameResult]:
        """
        批量编码（含可选 OCR）。

        本方法同时处理图片与视频：
          - 图片：作为单帧 (timestamp=0.0) 处理；
          - 视频：调用 extract_video_frames 生成器按子批次流式处理，
            内存占用恒定（不再一次性堆 22GB 进 frame_tasks）。
        """
        paths = req.batch.file_paths
        ocr_cfg = req.batch.ocr_config
        enable_ocr = ocr_cfg.enable_ocr if ocr_cfg else False
        ocr_langs = list(ocr_cfg.languages) if (ocr_cfg and ocr_cfg.languages) else list(DEFAULT_OCR_LANGS)

        print(f"[BATCH] {len(paths)} paths (OCR: {enable_ocr}, langs: {ocr_langs})")

        result_frames: list[framescout_pb2.FrameResult] = []

        for p in paths:
            try:
                if is_video_file(p):
                    # 视频流式处理（P0-6）：生成器 yield 关键帧，按子批次编码
                    frame_gen = extract_video_frames(p)
                    sub_batch: list[tuple[float, np.ndarray]] = []
                    yielded = 0
                    for ts, frame_rgb in frame_gen:
                        sub_batch.append((ts, frame_rgb))
                        yielded += 1
                        if len(sub_batch) >= MAX_FRAMES_PER_ENCODING_BATCH:
                            self._process_frame_sub_batch(p, sub_batch, enable_ocr, ocr_langs, result_frames)
                            sub_batch = []
                    # 收尾批次
                    if sub_batch:
                        self._process_frame_sub_batch(p, sub_batch, enable_ocr, ocr_langs, result_frames)
                    print(f"   [VIDEO] {p}: processed {yielded} keyframes")
                else:
                    # 图片：单帧
                    pil_img = load_image_pil(p)
                    frame_rgb = np.array(pil_img)
                    self._process_frame_sub_batch(p, [(0.0, frame_rgb)], enable_ocr, ocr_langs, result_frames)
            except Exception as e:
                # 单个文件读取/抽帧失败不应让整个 batch 崩溃
                print(f"⚠️ Failed to read {p}: {e}")
                continue

        return result_frames

    def _handle_ocr_task(self, req: framescout_pb2.EncodeRequest) -> list[framescout_pb2.FrameResult]:
        """按需 OCR 任务（只做文字识别，不编码向量）。"""
        paths = req.ocr_task.file_paths
        # 优先读内嵌的 ocr_config（协议演进方向）；旧客户端只填 languages 时回退之
        cfg = req.ocr_task.ocr_config if req.ocr_task.HasField("ocr_config") else None
        if cfg and cfg.languages:
            langs = list(cfg.languages)
        elif req.ocr_task.languages:
            langs = list(req.ocr_task.languages)
        else:
            langs = list(DEFAULT_OCR_LANGS)
        print(f"[OCR_TASK] {len(paths)} files, langs: {langs}")

        result_frames = []
        for p in paths:
            extracted = ""
            lines: list[dict] = []
            try:
                img_rgb = load_image(p)
                lines = self.ocr_manager.ocr_frame(img_rgb, langs)
                extracted = " ".join(line["text"] for line in lines)
            except Exception as e:
                print(f"⚠️ OCR failed for {p}: {e}")

            result_frames.append(self._make_frame_result(
                timestamp=0.0, vector=[], ocr_text=extracted, file_path=p, ocr_lines=lines
            ))
        return result_frames

    def _handle_file(self, req: framescout_pb2.EncodeRequest) -> list[framescout_pb2.FrameResult]:
        """单个文件（图片或视频）处理。"""
        task = req.file_task
        file_path = task.file_path
        is_video = is_video_file(file_path)
        print(f"[FILE] Processing: {file_path} (video={is_video})")

        # OCR 配置内嵌在载荷里（v3.2.0 结构修正）。
        # 未配置时保持历史行为：始终 OCR、默认语言。
        if task.HasField("ocr_config"):
            cfg = task.ocr_config
            enable_ocr = cfg.enable_ocr if cfg.HasField("enable_ocr") else True
            ocr_langs = list(cfg.languages) if cfg.languages else list(DEFAULT_OCR_LANGS)
        else:
            enable_ocr = True
            ocr_langs = list(DEFAULT_OCR_LANGS)

        result_frames: list[framescout_pb2.FrameResult] = []

        if is_video:
            # 视频流式子批次处理（P0-6）：与 _handle_batch 同样按子批次编码，
            # 内存占用恒定。生成器 yield 关键帧，边抽边编码边组装。
            frame_gen = extract_video_frames(file_path)
            sub_batch: list[tuple[float, np.ndarray]] = []
            for ts, frame_rgb in frame_gen:
                sub_batch.append((ts, frame_rgb))
                if len(sub_batch) >= MAX_FRAMES_PER_ENCODING_BATCH:
                    self._process_frame_sub_batch(file_path, sub_batch, enable_ocr, ocr_langs, result_frames)
                    sub_batch = []
            if sub_batch:
                self._process_frame_sub_batch(file_path, sub_batch, enable_ocr, ocr_langs, result_frames)
        else:
            # 图片：单帧
            try:
                img_rgb = load_image(file_path)
                self._process_frame_sub_batch(file_path, [(0.0, img_rgb)], enable_ocr, ocr_langs, result_frames)
            except Exception as e:
                print(f"⚠️ Failed to load {file_path}: {e}")

        return result_frames

    def _handle_text(self, req: framescout_pb2.EncodeRequest) -> list[framescout_pb2.FrameResult]:
        """文本查询编码，支持 PING 心跳与 BGE-M3 文本查询。"""
        if req.text == "PING_ENGINE":
            print("[PING] Health check OK")
            return []  # 返回空 frames 表示成功

        print(f"[TEXT] Query: {req.text}")

        # model 字段决定走哪条编码通道（两条检索公路，绝不混算）：
        #   BGE_M3_DENSE → BGE-M3 文本向量（语义查 OCR 文本，返回 dense_vector）
        #   否则（默认 SIGLIP_768）→ SigLIP text encoder（以文搜图，视觉检索）
        if req.model == framescout_pb2.EmbeddingModel.BGE_M3_DENSE:
            dense = self.bge.embed_texts([req.text])[0]
            if not dense:
                print("⚠️ [TEXT] BGE-M3 text engine unavailable or empty query.")
                return []  # 空 frames：Rust 侧据此报「文本引擎不可用」
            return [self._make_frame_result(
                timestamp=0.0, vector=[], ocr_text="", file_path="",
                dense_vector=dense,
            )]

        vector = self.siglip.embed_text(req.text)
        return [self._make_frame_result(
            timestamp=0.0, vector=vector, ocr_text="", file_path=""
        )]

    def _handle_text_task(self, req: framescout_pb2.EncodeRequest) -> list[framescout_pb2.TextEntry]:
        """纯文本条目入库：BGE-M3 编码文本，返回 TextEntry（dense，sparse 暂缓）。"""
        import uuid

        task = req.text_task
        print(f"[TEXT_TASK] source={task.source_uri}, len={len(task.text)}")

        dense = self.bge.embed_texts([task.text])[0]
        if not dense:
            print("⚠️ [TEXT_TASK] BGE-M3 text engine unavailable or empty text.")
            return []

        entry = framescout_pb2.TextEntry(
            entry_id=f"text_{uuid.uuid4().hex}",
            source_uri=task.source_uri,
            content=task.text,
            index_time=time.time(),
        )
        # metadata 是 map<string,string>，直接合并调用方传入的标签
        entry.metadata.update(task.metadata)
        entry.embedding.dense.extend(dense)
        # sparse 暂缓（留给 FTS5 trigram 批次做精确匹配）
        return [entry]

    # ------------------------------------------------------------------
    # 工具方法
    # ------------------------------------------------------------------
    def _make_frame_result(
        self,
        timestamp: float,
        vector: list[float],
        ocr_text: str,
        file_path: str,
        ocr_lines: list[dict] | None = None,
        frame_timestamp_ms: int = 0,
        dense_vector: list[float] | None = None,
    ) -> framescout_pb2.FrameResult:
        """
        构造一个 framescout_pb2.FrameResult。

        embedding_model 标记的是**视觉向量 `vector`** 出自哪个模型（SigLIP 768），
        必须显式标注：库内会同时存在多个模型的向量，未标注的行在检索时无法判断
        该按哪个模型过滤，而跨模型算相似度结果全是垃圾。

        `dense_vector` 是**独立的文本向量**（BGE-M3 dense 1024），非空即表示该帧
        的 OCR 文字已抽成文本向量，Rust 侧据此写入 media_text_vectors（model='bge-m3'）。
        它与 `vector`（视觉）是两条检索公路，不共用 embedding_model 标记。

        ocr_lines 是**行级**结果：检索与定位以它为真数据源，
        ocr_text 只是拼接全文（保留给旧搜索管道）。
        """
        frame_res = framescout_pb2.FrameResult(
            timestamp=timestamp,
            index_time=time.time(),
            ocr_text=ocr_text,
            file_path=file_path,
            embedding_model=framescout_pb2.EmbeddingModel.SIGLIP_768,
            ocr_engine=OCR_ENGINE_NAME,
            ocr_model=OCR_MODEL_NAME,
        )
        frame_res.vector.extend(vector)

        # 文本向量只在该帧确有可编码文字时填充；空则不写（Rust 侧据此跳过落库）
        if dense_vector:
            frame_res.dense_vector.extend(dense_vector)

        for line in ocr_lines or []:
            left, top, right, bottom = line["bbox"]
            frame_res.ocr_lines.add(
                text=line["text"],
                bbox_left=left,
                bbox_top=top,
                bbox_right=right,
                bbox_bottom=bottom,
                score=line["score"],
                lang=line["lang"],
                timestamp_ms=frame_timestamp_ms,
                payload_json=lines_to_payload(line["corners"]),
            )
        return frame_res

    def _send_error(self, task_id: str, message: str, code: int = 500) -> None:
        """发送错误响应。"""
        err = framescout_pb2.ErrorInfo(code=code, message=message, context="ai_inference")
        response = framescout_pb2.EncodeResponse(task_id=task_id, error=err)
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
                req = framescout_pb2.EncodeRequest()
                req.ParseFromString(raw_req)
                task_id = req.task_id

                payload_type = req.WhichOneof("payload")
                print(f"📨 Request type: {payload_type}, task_id: {req.task_id}")

                # 路由分发
                if payload_type == "batch":
                    success = framescout_pb2.SuccessPayload(frames=self._handle_batch(req))
                elif payload_type == "ocr_task":
                    success = framescout_pb2.SuccessPayload(frames=self._handle_ocr_task(req))
                elif payload_type == "file_task":
                    success = framescout_pb2.SuccessPayload(frames=self._handle_file(req))
                elif payload_type == "text":
                    success = framescout_pb2.SuccessPayload(frames=self._handle_text(req))
                elif payload_type == "text_task":
                    # 纯文本条目入库：返回 text_entries 而非 frames
                    success = framescout_pb2.SuccessPayload(text_entries=self._handle_text_task(req))
                else:
                    print(f"⚠️ Unknown payload: {payload_type}")
                    self._send_error(req.task_id, "Unknown payload type", code=400)
                    continue

                # 发送成功响应
                response = framescout_pb2.EncodeResponse(task_id=req.task_id, success=success)
                self.socket.send(response.SerializeToString())

            except Exception as e:
                traceback.print_exc()
                print(f"❌ Fatal error: {e}")
                # REP 套接字在 recv 之后必须 reply，否则后续请求会全部卡死
                self._send_error(task_id, str(e), code=500)