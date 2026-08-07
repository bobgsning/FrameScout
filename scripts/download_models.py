#!/usr/bin/env python3
"""
download_models.py — FrameScout Model Downloader (ONNX & SigLIP 2)

Downloads SigLIP 2 and EasyOCR models, and exports SigLIP 2 to ONNX format
for cross-hardware GPU acceleration (NVIDIA, AMD, Intel via DirectML).

Usage:
    python scripts/download_models.py
"""

import os
import sys
import torch

# Resolve project root (this script lives in FrameScout/scripts/)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
MODELS_DIR = os.path.join(PROJECT_ROOT, "src", "inference-worker", "models")


def extract_tensor_features(outputs):
    """
    🌟 Universal Tensor Smart Unpacker:
    Accurately extract Tensors from various HuggingFace ModelOutput/BaseModelOutputWithPooling objects.
    """

    if isinstance(outputs, torch.Tensor):
        return outputs
    if hasattr(outputs, "image_embeds") and outputs.image_embeds is not None:
        return outputs.image_embeds
    if hasattr(outputs, "text_embeds") and outputs.text_embeds is not None:
        return outputs.text_embeds
    if hasattr(outputs, "pooler_output") and outputs.pooler_output is not None:
        return outputs.pooler_output
    if hasattr(outputs, "last_hidden_state") and outputs.last_hidden_state is not None:
        return outputs.last_hidden_state
    if isinstance(outputs, (tuple, list)) and len(outputs) > 0:
        return outputs[0]
    return outputs


def export_siglip_to_onnx():
    """Download Google SigLIP 2 and export vision/text components to ONNX."""
    try:
        from transformers import AutoProcessor, AutoModel
    except ImportError:
        print("❌ transformers not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    siglip_dir = os.path.join(MODELS_DIR, "siglip2-base")
    os.makedirs(siglip_dir, exist_ok=True)

    model_name = "google/siglip2-base-patch16-256"
    print(f"⏳ Downloading SigLIP 2 model ({model_name})...")
    print(f"   Target directory: {siglip_dir}")

    processor = AutoProcessor.from_pretrained(model_name)
    model = AutoModel.from_pretrained(model_name)
    model.eval()

    # Save tokenizer/processor config for inference worker
    processor.save_pretrained(siglip_dir)

    print("⚡ Exporting SigLIP 2 components to ONNX format...")

    # 1. Export Vision Model to ONNX (Guaranteed 2D Output: [batch_size, 768])
    class VisionWrapper(torch.nn.Module):
        def __init__(self, model):
            super().__init__()
            self.model = model

        def forward(self, pixel_values):
            raw_outputs = self.model.get_image_features(pixel_values=pixel_values)
            features = extract_tensor_features(raw_outputs)

            # If the output contains Patch dimensions [B, 256, 768], average pool the Patch to reduce to [B, 768]
            if features.ndim == 3:
                features = features.mean(dim=1)

            # L2 normalize inside ONNX graph
            return features / features.norm(p=2, dim=-1, keepdim=True)

    vision_onnx_path = os.path.join(siglip_dir, "siglip2_vision.onnx")
    dummy_pixel_values = torch.randn(1, 3, 256, 256)
    vision_wrapper = VisionWrapper(model)

    torch.onnx.export(
        vision_wrapper,
        dummy_pixel_values,
        vision_onnx_path,
        input_names=["pixel_values"],
        output_names=["image_features"],
        dynamic_axes={
            "pixel_values": {0: "batch_size"},
            "image_features": {0: "batch_size"},
        },
        opset_version=14,
        dynamo=False,
    )
    print(f"   ✅ Vision ONNX saved (Guaranteed 768D): {vision_onnx_path}")

    # 2. Export Text Model to ONNX (Guaranteed 2D Output: [batch_size, 768])
    class TextWrapper(torch.nn.Module):
        def __init__(self, model):
            super().__init__()
            self.model = model

        def forward(self, input_ids):
            raw_outputs = self.model.get_text_features(input_ids=input_ids)
            features = extract_tensor_features(raw_outputs)

            # If the output contains Sequence dimensions [B, SeqLen, 768], average pool the Sequence to reduce to [B, 768]
            if features.ndim == 3:
                features = features.mean(dim=1)

            # L2 normalize inside ONNX graph
            return features / features.norm(p=2, dim=-1, keepdim=True)

    text_onnx_path = os.path.join(siglip_dir, "siglip2_text.onnx")
    dummy_input_ids = torch.randint(0, 1000, (1, 64), dtype=torch.long)
    text_wrapper = TextWrapper(model)

    torch.onnx.export(
        text_wrapper,
        dummy_input_ids,
        text_onnx_path,
        input_names=["input_ids"],
        output_names=["text_features"],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "text_features": {0: "batch_size"},
        },
        opset_version=14,
        dynamo=False,
    )
    print(f"   ✅ Text ONNX saved (Guaranteed 768D): {text_onnx_path}")


def download_easyocr():
    """Download EasyOCR English model (~100 MB)."""
    try:
        import easyocr
    except ImportError:
        print("❌ easyocr not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    ocr_dir = os.path.join(MODELS_DIR, "easyocr")
    os.makedirs(ocr_dir, exist_ok=True)

    print("⏳ Downloading EasyOCR English model...")
    print(f"   Target directory: {ocr_dir}")

    reader = easyocr.Reader(["en"], model_storage_directory=ocr_dir)
    del reader
    print(f"✅ EasyOCR model saved to {ocr_dir}")


if __name__ == "__main__":
    print("=" * 52)
    print("   FrameScout Model Downloader (ONNX + SigLIP 2)")
    print("=" * 52)
    print()

    try:
        export_siglip_to_onnx()
        print()
        download_easyocr()
        print()
        print("🎉 All models & ONNX computational graphs built successfully!")
        print(f"📁 Target location: {MODELS_DIR}")
        print("💎 FrameScout is now fully equipped for cross-hardware GPU acceleration!")
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"\n❌ An error occurred: {e}")
        sys.exit(1)
