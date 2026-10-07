#!/usr/bin/env python3
"""
download_models.py — FrameScout Model Downloader (ONNX & SigLIP 2)

Downloads SigLIP 2, exports it to ONNX for cross-hardware GPU acceleration,
and self-checks the RapidOCR engine (PP-OCRv4 ONNX weights ship inside the wheel,
so no download is needed for the default Chinese+English language set).

Usage:
    python scripts/download_models.py
"""

import os
import sys
import shutil
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
        import transformers
        from packaging import version as pkg_version
    except ImportError:
        print("❌ transformers not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    # 版本一致性校验：运行 venv 已锁定 transformers 4.x 且 >=4.50（build_nuitka.ps1
    # -NoTorch 剔除 torch 的前提）。导出环境必须同用 4.x 且 >=4.50：
    #   - 5.x：processor 配置命名（processor_config.json）与 tokenizer 文件集与 4.x 不兼容；
    #   - 4.49：有 SigLIP 2 processor 映射 bug（硬编码 SiglipTokenizer 去加载 GemmaTokenizer，
    #           报 vocab_file=None），AutoProcessor 加载 SigLIP 2 必崩。
    # 从源头保证导出与运行两边文件一致。
    _ver = pkg_version.parse(transformers.__version__)
    if _ver < pkg_version.parse("4.50.0") or _ver >= pkg_version.parse("5.0.0"):
        print(
            "❌ 导出环境 transformers 为 "
            + transformers.__version__
            + "，与运行 venv 锁定的 4.x（>=4.50）不兼容。\n"
            "   请将导出环境的 transformers 装到 >=4.50,<5（如 pip install 'transformers==4.57.6'）后重跑本脚本，\n"
            "   否则运行 venv 在 4.x 下加载 SigLIP processor 会失败（4.49 映射 bug 或 5.x 配置不兼容）。"
        )
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

    # 4.x 兼容名防御：个别版本仍写出 processor_config.json，复制一份
    # preprocessor_config.json 供 4.x image processor 识别。
    _pc = os.path.join(siglip_dir, "processor_config.json")
    _ppc = os.path.join(siglip_dir, "preprocessor_config.json")
    if os.path.exists(_pc) and not os.path.exists(_ppc):
        shutil.copyfile(_pc, _ppc)
        print(f"   ✅ 兼容名已生成: {os.path.basename(_ppc)}")

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


def check_rapidocr():
    """
    Self-check RapidOCR: the PP-OCRv4 ONNX weights ship inside the
    rapidocr_onnxruntime wheel, so there is nothing to download for the default
    language set (en + ch_sim). We only verify it loads and report which
    ONNX providers are actually usable (DirectML vs CPU fallback).
    """
    try:
        from rapidocr_onnxruntime import RapidOCR
    except ImportError:
        print("❌ rapidocr_onnxruntime not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    import onnxruntime as ort

    print("⏳ Checking RapidOCR availability...")
    print(f"   ONNX Runtime version : {ort.__version__}")
    print(f"   Available providers  : {ort.get_available_providers()}")

    # 自检：先尝试 DML，不可用则回退 CPU。这里只验证引擎能构造，
    # 不真正跑图——真正的「三组对比验收」见文档 6.4，需要真实语料。
    try:
        RapidOCR(det_use_dml=True, cls_use_dml=True, rec_use_dml=True)
        print("   ✅ RapidOCR ready (DirectML path attempted)")
    except Exception as e:
        print(f"   ⚠️ DirectML init failed ({e}); falling back to CPU")
        RapidOCR()
        print("   ✅ RapidOCR ready (CPU path)")

    print("✅ RapidOCR self-check passed (PP-OCRv4 weights ship with the wheel).")


def export_bge_m3_to_onnx():
    """
    Download BAAI/bge-m3 and export its **dense** encoder to ONNX.

    BGE-M3 dense = L2-normalize(CLS token embedding) of the XLM-R backbone.
    只导出 dense（文本通道当前策略）；sparse / colbert 头**不导出**——
    sparse 留给 FTS5 trigram 那批做精确匹配（见战略规划书 4.6 / 5.4）。

    ⚠️ 本函数需要 torch + transformers 环境（一次性导出用），
    与 inference-worker 的运行 venv（onnxruntime-directml，无 torch）分离。
    """
    try:
        from transformers import AutoTokenizer, AutoModel
    except ImportError:
        print("❌ transformers not installed. Run: pip install transformers torch")
        sys.exit(1)

    bge_dir = os.path.join(MODELS_DIR, "bge-m3")
    os.makedirs(bge_dir, exist_ok=True)

    # 清理上次导出失败留下的碎片（onnx__MatMul* / model.*.weight / Constant_* / *.onnx），
    # 避免新旧文件混杂。tokenizer 文件随后会重新 save_pretrained 覆盖。
    for _name in os.listdir(bge_dir):
        _p = os.path.join(bge_dir, _name)
        if _name.startswith("onnx__") or _name.startswith("model.") or _name.startswith("Constant_") or _name.endswith(".onnx"):
            if os.path.isfile(_p):
                os.remove(_p)
            elif os.path.isdir(_p):
                shutil.rmtree(_p)

    model_name = "BAAI/bge-m3"
    print(f"⏳ Downloading BGE-M3 model ({model_name})...")
    print(f"   Target directory: {bge_dir}")

    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = AutoModel.from_pretrained(model_name)
    model.eval()

    # 保存 tokenizer，worker 推理时用它做 tokenize（与导出时同源，杜绝漂移）
    tokenizer.save_pretrained(bge_dir)

    print("⚡ Exporting BGE-M3 dense encoder to ONNX...")

    class DenseWrapper(torch.nn.Module):
        def __init__(self, model):
            super().__init__()
            # float16 减半：BGE-M3 dense 约 2.2GB（float32），超过 ONNX protobuf 2GB
            # 上限，dynamo=False 导出会失败并留下外部数据碎片（onnx__MatMul* /
            # model.*.weight 等）。转 float16 后 ~1.1GB 落在 2GB 内；检索余弦相似度
            # 对 float16 精度不敏感。
            self.model = model.half()

        def forward(self, input_ids, attention_mask):
            outputs = self.model(input_ids=input_ids, attention_mask=attention_mask)
            # CLS token embedding：BGE-M3 dense 官方 pooling 即 CLS token
            cls = outputs.last_hidden_state[:, 0, :]  # [B, 1024]
            # L2 normalize inside ONNX graph（与 SigLIP 导出同一套路）
            return cls / cls.norm(p=2, dim=-1, keepdim=True)

    dense_onnx_path = os.path.join(bge_dir, "bge_m3_dense.onnx")
    dummy_input_ids = torch.randint(0, 250000, (1, 16), dtype=torch.long)
    dummy_attention_mask = torch.ones((1, 16), dtype=torch.long)
    wrapper = DenseWrapper(model)

    torch.onnx.export(
        wrapper,
        (dummy_input_ids, dummy_attention_mask),
        dense_onnx_path,
        input_names=["input_ids", "attention_mask"],
        output_names=["dense"],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "dense": {0: "batch_size"},
        },
        opset_version=14,
        dynamo=False,
    )
    print(f"   ✅ BGE-M3 dense ONNX saved (1024D): {dense_onnx_path}")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="FrameScout model downloader & ONNX exporter"
    )
    parser.add_argument(
        "--bge",
        action="store_true",
        help="Export BGE-M3 dense encoder only (text channel); requires torch + transformers",
    )
    args = parser.parse_args()

    # --bge：只导出 BGE-M3 dense（文本通道），不重复下载 SigLIP。
    if args.bge:
        try:
            export_bge_m3_to_onnx()
            print()
            print(f"🎉 BGE-M3 dense ONNX exported to: {MODELS_DIR}")
        except Exception as e:
            import traceback
            traceback.print_exc()
            print(f"\n❌ An error occurred: {e}")
            sys.exit(1)
        sys.exit(0)

    print("=" * 52)
    print("   FrameScout Model Downloader (ONNX + SigLIP 2)")
    print("=" * 52)
    print()

    try:
        export_siglip_to_onnx()
        print()
        check_rapidocr()
        print()
        print("🎉 All models & ONNX computational graphs built successfully!")
        print(f"📁 Target location: {MODELS_DIR}")
        print("💎 FrameScout is now fully equipped for cross-hardware GPU acceleration!")
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"\n❌ An error occurred: {e}")
        sys.exit(1)
