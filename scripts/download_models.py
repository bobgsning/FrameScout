#!/usr/bin/env python3
"""
download_models.py — FrameScout Model Downloader

Downloads CLIP and EasyOCR models into src/inference-worker/models/.
Run once before first launch. Requires internet access.

Usage:
    python scripts/download_models.py
"""

import os
import sys

# Resolve project root (this script lives in FrameScout/scripts/)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
MODELS_DIR = os.path.join(PROJECT_ROOT, "src", "inference-worker", "models")


def download_clip():
    """Download OpenAI CLIP ViT-B/32 (~600 MB)."""
    try:
        from transformers import CLIPProcessor, CLIPModel
    except ImportError:
        print("❌ transformers not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    clip_dir = os.path.join(MODELS_DIR, "openai-clip")
    os.makedirs(clip_dir, exist_ok=True)

    print("⏳ Downloading CLIP model (openai/clip-vit-base-patch32)...")
    print(f"   Target directory: {clip_dir}")

    model_name = "openai/clip-vit-base-patch32"
    processor = CLIPProcessor.from_pretrained(model_name)
    model = CLIPModel.from_pretrained(model_name)

    processor.save_pretrained(clip_dir)
    model.save_pretrained(clip_dir)
    print(f"✅ CLIP model saved to {clip_dir}")


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

    # Initializing without download_enabled=False forces download
    reader = easyocr.Reader(["en"], model_storage_directory=ocr_dir)
    del reader  # Free memory
    print(f"✅ EasyOCR model saved to {ocr_dir}")


if __name__ == "__main__":
    print("=" * 48)
    print("   FrameScout Model Downloader")
    print("=" * 48)
    print()

    try:
        download_clip()
        print()
        download_easyocr()
        print()
        print("🎉 All models downloaded successfully!")
        print(f"📁 Models are located at: {MODELS_DIR}")
        print()
        print("⚠️  After this one-time download, FrameScout runs 100% offline.")
    except Exception as e:
        print(f"\n❌ An error occurred: {e}")
        print("Please check your internet connection and try again.")
        sys.exit(1)