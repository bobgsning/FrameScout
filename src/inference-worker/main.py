"""
FrameScout Inference Worker
============================
Python process that handles all AI inference:
- CLIP image & text embeddings (via Transformers)
- EasyOCR text extraction
- OpenCV video frame extraction

Communicates with Rust/Tauri core via ZeroMQ + Protobuf.
Listens on tcp://127.0.0.1:5555

Author: bobgsning
"""
# SPDX-License-Identifier: Apache-2.0

import os
import sys
from transformers import CLIPProcessor, CLIPModel

# ⚠️ Offline mode: prevent any network access
os.environ["TRANSFORMERS_OFFLINE"] = "1"
os.environ["HF_HUB_OFFLINE"] = "1"

import zmq
import search_pb2
import torch
import cv2
import easyocr
import numpy as np
from PIL import Image

import datetime
import tempfile

# ============================================================
#  Device Selection
# ============================================================
def get_optimal_device():
    """Auto-detect CUDA GPU, fall back to CPU."""
    if torch.cuda.is_available():
        print("🔥 [Compute Core] NVIDIA GPU detected! CUDA acceleration enabled.")
        return torch.device("cuda")
    print("🐌 [Compute Core] No NVIDIA GPU detected. Using CPU mode.")
    return torch.device("cpu")


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
    # Redirect stdout/stderr to log file for debugging
    log_filename = f'ai_worker_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log'
    log_path = os.path.join(tempfile.gettempdir(), log_filename)
    sys.stdout = open(log_path, 'w', encoding='utf-8', buffering=1)
    sys.stderr = sys.stdout

    # ZeroMQ REP socket — synchronous request/reply
    context = zmq.Context()
    socket = context.socket(zmq.REP)
    socket.bind("tcp://127.0.0.1:5555")
    
    device = get_optimal_device()

    # Resolve model paths relative to executable
    if getattr(sys, 'frozen', False):
        base_path = os.path.dirname(sys.executable)
    else:
        base_path = os.path.dirname(os.path.abspath(__file__))

    clip_path = os.path.join(base_path, "models", "openai-clip")
    ocr_path = os.path.join(base_path, "models", "easyocr")

    # Sanity check: CLIP model must exist
    if not os.path.exists(clip_path):
        print(f"\n❌ [FATAL ERROR] Cannot find CLIP model folder at: {clip_path}")
        print("💡 Solution: Please ensure you copied the 'models' folder!")
        sys.exit(1)
        
    os.makedirs(ocr_path, exist_ok=True)

    print(f"⏳ Waking up Visual Engine from: {clip_path}")
    processor = CLIPProcessor.from_pretrained(clip_path)
    model = CLIPModel.from_pretrained(clip_path).to(device)
    
    print(f"⏳ Waking up OCR Engine from: {ocr_path}")
    try:
        reader = easyocr.Reader(
            ['en'], 
            gpu=torch.cuda.is_available(), 
            model_storage_directory=ocr_path, 
            download_enabled=False
        )
    except Exception as e:
        print(f"\n❌ [FATAL ERROR] EasyOCR models missing in: {ocr_path}")
        print("💡 Solution: Please copy OCR model files into the easyocr folder!")
        sys.exit(1)

    print("🚀 [AI Worker] Dual-engine online! Listening on port 5555...")

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

                    # Batch CLIP inference
                    inputs = processor(images=pil_images, return_tensors="pt", padding=True)
                    inputs = {k: v.to(device) for k, v in inputs.items()}

                    with torch.no_grad():
                        outputs = model.get_image_features(**inputs)
                        
                        # Handle different return types
                        if not isinstance(outputs, torch.Tensor):
                            if hasattr(outputs, "image_embeds"):
                                image_features = outputs.image_embeds
                            elif hasattr(outputs, "pooler_output"):
                                image_features = outputs.pooler_output
                            else:
                                image_features = outputs[0]
                        else:
                            image_features = outputs

                        # L2 normalize
                        image_features = image_features / image_features.norm(p=2, dim=-1, keepdim=True)
                        vectors = image_features.cpu().tolist()

                    # Assemble response
                    for p, vec, ocr in zip(valid_paths, vectors, ocr_texts):
                        frame_res = search_pb2.FrameResult(
                            timestamp=0.0,
                            vector=vec,
                            ocr_text=ocr,
                            file_path=p
                        )
                        result_frames.append(frame_res)

            # ---- Single File (image or video) ----
            elif payload_type == "file_path":
                ext = os.path.splitext(req.file_path)[-1].lower()
                is_video = ext in ['.mp4', '.mov', '.avi', '.mkv']
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
                    inputs = processor(images=pil_img, return_tensors="pt")
                    inputs = {k: v.to(device) for k, v in inputs.items()}
                    
                    with torch.no_grad():
                        outputs = model.get_image_features(**inputs)
                        if not isinstance(outputs, torch.Tensor):
                            if hasattr(outputs, "image_embeds"):
                                image_features = outputs.image_embeds
                            elif hasattr(outputs, "pooler_output"):
                                image_features = outputs.pooler_output
                            else:
                                image_features = outputs[0]
                        else:
                            image_features = outputs
                        image_features = image_features / image_features.norm(p=2, dim=-1, keepdim=True)
                    
                    frame_res = search_pb2.FrameResult(
                        timestamp=timestamp, 
                        vector=image_features[0].cpu().tolist(),
                        ocr_text=extracted_text,
                        file_path=req.file_path
                    )
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
                inputs = processor(text=req.text, return_tensors="pt", padding=True)
                inputs = {k: v.to(device) for k, v in inputs.items()}
                
                with torch.no_grad():
                    outputs = model.get_text_features(**inputs)
                    if not isinstance(outputs, torch.Tensor):
                        if hasattr(outputs, "text_embeds"):
                            text_features = outputs.text_embeds
                        elif hasattr(outputs, "pooler_output"):
                            text_features = outputs.pooler_output
                        else:
                            text_features = outputs[0]
                    else:
                        text_features = outputs
                    text_features = text_features / text_features.norm(p=2, dim=-1, keepdim=True)

                frame_res = search_pb2.FrameResult(
                    timestamp=0.0, 
                    vector=text_features[0].cpu().tolist(), 
                    ocr_text="", 
                    file_path=""
                )
                result_frames.append(frame_res)

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
