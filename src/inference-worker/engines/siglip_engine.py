"""
SigLIP 2 特征提取引擎：封装 Vision & Text ONNX 推理。
"""
import os
import numpy as np
from transformers import AutoProcessor
from PIL import Image

from utils.onnx_utils import create_onnx_session
from utils.vector_utils import format_vector_output
from config import EMBEDDING_DIM


class SiglipEngine:
    """
    SigLIP 2 双编码器封装。
    负责加载 processor、构建 ONNX sessions，并输出标准 768D 向量。
    """

    def __init__(self, model_dir: str):
        """
        Args:
            model_dir: 包含 siglip2_vision.onnx 和 siglip2_text.onnx 的目录
        """
        vision_onnx = os.path.join(model_dir, "siglip2_vision.onnx")
        text_onnx = os.path.join(model_dir, "siglip2_text.onnx")

        # 检查文件是否存在
        if not os.path.exists(vision_onnx) or not os.path.exists(text_onnx):
            raise FileNotFoundError(
                f"SigLIP 2 ONNX models missing in {model_dir}. "
                "Please run 'python scripts/download_models.py' first."
            )

        print(f"⏳ Loading SigLIP 2 processor from: {model_dir}")
        self.processor = AutoProcessor.from_pretrained(model_dir)

        print("⏳ Initializing SigLIP 2 Vision ONNX Engine...")
        self.vision_session = create_onnx_session(vision_onnx)

        print("⏳ Initializing SigLIP 2 Text ONNX Engine...")
        self.text_session = create_onnx_session(text_onnx)

        # 获取模型输入名称（可能随模型变化，动态获取）
        self.vision_input_name = self.vision_session.get_inputs()[0].name
        self.text_input_name = self.text_session.get_inputs()[0].name

    def embed_images(self, images: list) -> list[list[float]]:
        """
        输入一组 PIL.Image，输出 List of 768D 向量 (List[List[float]])。
        """
        if not images:
            return []

        # 预处理为 numpy
        inputs = self.processor(images=images, return_tensors="np")
        pixel_values = inputs["pixel_values"].astype(np.float32)

        onnx_inputs = {self.vision_input_name: pixel_values}
        raw_vecs = self.vision_session.run(None, onnx_inputs)[0]

        # 强制归一化为 (N, 768)
        return format_vector_output(raw_vecs, target_dim=EMBEDDING_DIM)

    def embed_image(self, image: Image.Image) -> list[float]:
        """单张图片编码，返回一个 768D 向量（list of floats）。"""
        vecs = self.embed_images([image])
        return vecs[0]

    def embed_text(self, text: str) -> list[float]:
        """
        输入一段文本，输出 768D 向量（list of floats）。
        """
        inputs = self.processor(
            text=[text],
            return_tensors="np",
            padding="max_length",
            max_length=64
        )
        input_ids = inputs["input_ids"].astype(np.int64)

        onnx_inputs = {self.text_input_name: input_ids}
        raw_vec = self.text_session.run(None, onnx_inputs)[0]

        # 归一化
        vecs = format_vector_output(raw_vec, target_dim=EMBEDDING_DIM)
        return vecs[0]