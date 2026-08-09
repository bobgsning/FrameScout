"""
FrameScout Inference Worker (ONNX + SigLIP 2 Edition)
=====================================================
Python process that handles all AI inference using ONNX Runtime:
- Universal GPU Acceleration (NVIDIA / AMD / Intel DirectML + CUDA + CPU)
- SigLIP 2 image & text embeddings (768D)
- EasyOCR text extraction
- OpenCV video frame extraction

Listens on tcp://127.0.0.1:5555

Author: bobgsning
"""
# SPDX-License-Identifier: Apache-2.0

import os
import sys
import datetime
import tempfile
import numpy as np
import cv2
import zmq
import search_pb2
import easyocr
from PIL import Image
import time

# Transformers Processor for preprocessing
from transformers import AutoProcessor
import onnxruntime as ort

# ⚠️ Offline mode: prevent any network access
os.environ["TRANSFORMERS_OFFLINE"] = "1"
os.environ["HF_HUB_OFFLINE"] = "1"

# ============================================================
#  ONNX Session Setup (Hardware Execution Provider Auto-Detect)
# ============================================================
def create_onnx_session(onnx_path):
    """
    Creates ONNX Runtime session with automatic hardware selection:
    1. DirectML (AMD, NVIDIA, Intel GPUs on Windows)
    2. CUDA (NVIDIA GPU)
    3. CPU (Fallback)
    """
    available_providers = ort.get_available_providers()
    providers = []

    if "CUDAExecutionProvider" in available_providers:
        providers.append("CUDAExecutionProvider")
        print("🔥 NVIDIA CUDA acceleration enabled!")
    elif "DmlExecutionProvider" in available_providers:
        providers.append("DmlExecutionProvider")
        print("🔥 DirectML GPU acceleration enabled! (AMD/Intel)")
    
    providers.append("CPUExecutionProvider")

    sess_options = ort.SessionOptions()
    sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    return ort.InferenceSession(onnx_path, sess_options, providers=providers)


def format_vector_output(raw_vec, target_dim=768):
    """
    🌟 Ultimate Dimensional Safety Shield:
    Regardless of whether ONNX outputs (N, 768), (N, 256, 768), or 1D/3D arrays,
    it will be safely processed and converged to the standard List[List[float]] (N rows, 768 columns). 
    (Guaranteed 768D)
    """
    arr = np.array(raw_vec, dtype=np.float32)
    if arr.ndim == 3:
        arr = arr.mean(axis=1)  # Average pool the Patch dimension.
    elif arr.ndim == 1:
        arr = np.expand_dims(arr, axis=0)
    
    # Insurance protection: If the last dimension still doesn't equal 768
    if arr.shape[-1] != target_dim:
        arr = arr.reshape(arr.shape[0], -1, target_dim).mean(axis=1)
        
    return arr.tolist()


# ============================================================
#  Video Frame Extraction
# ============================================================
def extract_video_frames(video_path, extract_fps=1, max_duration=3600):
    """
    Extract frames from video at specified FPS.
    Default: 1 FPS, max 1 hour.
    
    TODO v3.2: Replace with scene-detection based extraction.
    """
    cap = cv2.VideoCapture(video_path)
    fps = cap.get(cv2.CAP_PROP_FPS)
    if fps == 0 or fps != fps:  # handle NaN
        fps = 25.0
    
    frames = []
    sec = 0.0
    while True:
        frame_id = int(fps * sec)
        cap.set(cv2.CAP_PROP_POS_FRAMES, frame_id)
        ret, frame = cap.read()
        if not ret:
            break
        frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        frames.append((sec, frame_rgb))
        sec += 1.0 / extract_fps
        if sec > max_duration:
            break
    cap.release()
    return frames


# ============================================================
#  Main Loop
# ============================================================
def main():
    # Create FrameScout_Global Folder in temp directory for logs
    log_dir = os.path.join(tempfile.gettempdir(), 'FrameScout_Global')
    os.makedirs(log_dir, exist_ok=True)

    # Redirect stdout/stderr to log file for debugging
    log_filename = f'ai_worker_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log'
    log_path = os.path.join(log_dir, log_filename)
    sys.stdout = open(log_path, 'w', encoding='utf-8', buffering=1)
    sys.stderr = sys.stdout

    # ZeroMQ REP socket — synchronous request/reply
    context = zmq.Context()
    socket = context.socket(zmq.REP)
    socket.bind("tcp://127.0.0.1:5555")

    # Resolve model paths relative to executable
    if getattr(sys, 'frozen', False):
        base_path = os.path.dirname(sys.executable)
    else:
        base_path = os.path.dirname(os.path.abspath(__file__))

    siglip_dir = os.path.join(base_path, "models", "siglip2-base")
    ocr_path = os.path.join(base_path, "models", "easyocr")

    vision_onnx = os.path.join(siglip_dir, "siglip2_vision.onnx")
    text_onnx = os.path.join(siglip_dir, "siglip2_text.onnx")

    # Sanity check: SigLIP 2 model must exist
    if not os.path.exists(vision_onnx) or not os.path.exists(text_onnx):
        print(f"\n❌ [FATAL ERROR] ONNX models missing in: {siglip_dir}")
        print("💡 Solution: Please run 'python scripts/download_models.py' first!")
        sys.exit(1)
        
    os.makedirs(ocr_path, exist_ok=True)

    print(f"⏳ Loading SigLIP 2 Processor from: {siglip_dir}")
    processor = AutoProcessor.from_pretrained(siglip_dir)

    print(f"⏳ Initializing ONNX Vision Engine...")
    vision_session = create_onnx_session(vision_onnx)

    print(f"⏳ Initializing ONNX Text Engine...")
    text_session = create_onnx_session(text_onnx)

    print(f"⏳ Initializing EasyOCR Engine from: {ocr_path}")
    try:
        reader = easyocr.Reader(
            ['en'], 
            gpu=True,  # EasyOCR will handle GPU check internally
            model_storage_directory=ocr_path, 
            download_enabled=False
        )
    except Exception:
        # Fallback to CPU for OCR if GPU init fails
        reader = easyocr.Reader(
            ['en'], 
            gpu=False, 
            model_storage_directory=ocr_path, 
            download_enabled=False
        )

    print("🚀 [AI Worker] ONNX Multi-Hardware Engine online! Listening on port 5555...")


    # Main request/reply loop
    while True:
        try:
            raw_req = socket.recv()
            req = search_pb2.EncodeRequest()
            req.ParseFromString(raw_req)
            
            payload_type = req.WhichOneof("payload")
            result_frames = []

            # ---- Batch Processing (folder scanning) ----
            if payload_type == "batch":
                paths = req.batch.file_paths
                print(f"[PYTHON] Batch request: {len(paths)} paths")
                
                pil_images = []
                valid_paths = []

                for p in paths:
                    try:
                        img_bgr = cv2.imdecode(np.fromfile(p, dtype=np.uint8), cv2.IMREAD_COLOR)
                        if img_bgr is not None:
                            img_rgb = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)
                            pil_images.append(Image.fromarray(img_rgb))
                            valid_paths.append(p)
                    except Exception as e:
                        print(f"⚠️ Failed to read image {p}: {e}")

                if pil_images:
                    # OCR (per-image to avoid OOM)
                    ocr_texts = []
                    for img_np in [np.array(img) for img in pil_images]:
                        ocr_res = reader.readtext(img_np, detail=0)
                        ocr_texts.append(" ".join(ocr_res))

                    # ONNX Vision Inference
                    inputs = processor(images=pil_images, return_tensors="np")
                    pixel_values = inputs["pixel_values"].astype(np.float32)

                    onnx_inputs = {vision_session.get_inputs()[0].name: pixel_values}
                    raw_vecs = vision_session.run(None, onnx_inputs)[0]
                    # 🌟 Force reshape to (N, 768), ensuring each element is a pure 768-dimension float list
                    vectors = format_vector_output(raw_vecs, target_dim=768)

                    for p, vec, ocr in zip(valid_paths, vectors, ocr_texts):
                        frame_res = search_pb2.FrameResult(
                            timestamp=0.0,
                            index_time=time.time(),   # 🌟 Add storage time
                            ocr_text=ocr,
                            file_path=p
                        )
                        frame_res.vector.extend(vec)  # 🌟 Use standard .extend to populate the list
                        result_frames.append(frame_res)

            # ---- Single File (image or video) ----
            elif payload_type == "file_path":
                ext = os.path.splitext(req.file_path)[-1].lower()
                is_video = ext in ['.mp4', '.mov', '.avi', '.mkv', '.webm', '.flv']

                print(f"🎞️ Processing: {req.file_path}")
                
                image_tasks = []
                if is_video:
                    image_tasks = extract_video_frames(req.file_path)
                else:
                    img_bgr = cv2.imdecode(np.fromfile(req.file_path, dtype=np.uint8), cv2.IMREAD_COLOR)
                    if img_bgr is not None:
                        img_rgb = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)
                        image_tasks = [(0.0, img_rgb)]

                for timestamp, frame_rgb in image_tasks:
                    ocr_results = reader.readtext(frame_rgb, detail=0)
                    extracted_text = " ".join(ocr_results)

                    if extracted_text:
                        print(f"   [T={timestamp}s] OCR: {extracted_text[:80]}")

                    pil_img = Image.fromarray(frame_rgb)
                    inputs = processor(images=pil_img, return_tensors="np")
                    pixel_values = inputs["pixel_values"].astype(np.float32)

                    onnx_inputs = {vision_session.get_inputs()[0].name: pixel_values}
                    raw_vector = vision_session.run(None, onnx_inputs)[0]
                    # 🌟 Force reshape to (N, 768), ensuring each element is a pure 768-dimension float list
                    vector1 = format_vector_output(raw_vector, target_dim=768)[0]

                    frame_res = search_pb2.FrameResult(
                        timestamp=timestamp,
                        index_time=time.time(),   # 🌟 Add storage time
                        ocr_text=extracted_text,
                        file_path=req.file_path
                    )
                    frame_res.vector.extend(vector1)  # 🌟 Use standard .extend to populate the list
                    result_frames.append(frame_res)

            # ---- Text Query ----
            elif payload_type == "text":
                if req.text == "PING_ENGINE":
                    # Health check — return empty success
                    success = search_pb2.SuccessPayload(frames=[])
                    response = search_pb2.EncodeResponse(task_id=req.task_id, success=success)
                    socket.send(response.SerializeToString())
                    continue
                    
                print(f"📝 Text query: {req.text}")
                inputs = processor(text=[req.text], return_tensors="np", padding="max_length", max_length=64)
                input_ids = inputs["input_ids"].astype(np.int64)

                onnx_inputs = {text_session.get_inputs()[0].name: input_ids}
                raw_vector = text_session.run(None, onnx_inputs)[0]
                # 🌟 Force reshape to (N, 768), ensuring each element is a pure 768-dimension float list
                vector2 = format_vector_output(raw_vector, target_dim=768)[0]

                frame_res = search_pb2.FrameResult(
                    timestamp=0.0,
                    index_time=time.time(),   # 🌟 Add storage time
                    ocr_text="", 
                    file_path=""
                )
                frame_res.vector.extend(vector2)  # 🌟 Use standard .extend to populate the list
                result_frames.append(frame_res)
                
            else:
                print(f"⚠️ Unknown payload type: {payload_type}")
                err = search_pb2.ErrorInfo(code=400, message="Unknown payload type", context="ai_inference")
                response = search_pb2.EncodeResponse(task_id=req.task_id, error=err)
                socket.send(response.SerializeToString())
                continue

            # Send success response
            success = search_pb2.SuccessPayload(frames=result_frames)
            response = search_pb2.EncodeResponse(task_id=req.task_id, success=success)
            socket.send(response.SerializeToString())
            
        except Exception as e:
            import traceback
            traceback.print_exc()
            print(f"❌ Fatal error: {e}")
            err = search_pb2.ErrorInfo(code=500, message=str(e), context="ai_inference")
            response = search_pb2.EncodeResponse(task_id=req.task_id, error=err)
            socket.send(response.SerializeToString())


if __name__ == "__main__":
    main()