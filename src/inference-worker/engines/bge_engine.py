"""
BGE-M3 文本向量引擎：dense-only 封装（sparse 暂缓，留给 FTS5 trigram 批次）。

============================================================
 定位：文本界的 SigLIP —— 第二条检索公路
============================================================
 SigLIP 2 是视觉向量（768D），只懂画面；BGE-M3 是文本向量（dense 1024D），
 负责「用文字搜媒体」。两者互补、绝不混算（混库禁忌见 proto EmbeddingModel）。

 本轮策略（已与用户确认）：
  · **只落 dense**：media_text_vectors.dense_blob 填 1024 维 BGE-M3 dense；
  · **sparse 暂缓**：词->权重留空，等 FTS5 trigram 那批一起做精确匹配；
  · dense = normalize(CLS token embedding)，已在 ONNX 导出时内置（见
    scripts/download_models.py::export_bge_m3_to_onnx），推理侧无需再算。

============================================================
 抽象层的意义
============================================================
 `embed_texts()` 是对外唯一接口：输入文本列表，返回 dense 向量列表。
 将来换模型（BGE-M4 / Qwen3-Embedding）只改这一层，ZMQ 与 proto 完全不动。
"""
from __future__ import annotations

import os

import numpy as np

from config import BGE_DENSE_DIM, BGE_MAX_BATCH, BGE_MAX_TOKENS
from utils.onnx_utils import create_onnx_session


class BgeEngine:
    """
    BGE-M3 dense 推理引擎。

    模型缺省时不强制启动失败——文本通道是**增量能力**，不影响视觉主链路：
    加载失败只禁用文本编码，视觉检索照常工作（系统有感知，但不越权中断）。
    """

    def __init__(self, model_dir: str):
        self.model_dir = model_dir
        self._available = False
        self._session = None
        self._tokenizer = None
        self._try_load()

    # ------------------------------------------------------------------
    # 加载
    # ------------------------------------------------------------------
    def _try_load(self):
        dense_onnx = os.path.join(self.model_dir, "bge_m3_dense.onnx")
        if not os.path.exists(dense_onnx):
            print(f"⚠️ [BGE] model not found at {dense_onnx}; text channel disabled.")
            print("   Export it with: python scripts/download_models.py --bge")
            return
        try:
            from transformers import AutoTokenizer

            print(f"⏳ [BGE] Loading tokenizer from: {self.model_dir}")
            self._tokenizer = AutoTokenizer.from_pretrained(self.model_dir)

            print("⏳ [BGE] Initializing dense ONNX engine...")
            self._session = create_onnx_session(dense_onnx)
            self._available = True
            print(f"✅ [BGE] dense engine ready (dim={BGE_DENSE_DIM}).")
        except Exception as e:
            print(f"⚠️ [BGE] init failed, text channel disabled: {e}")

    @property
    def available(self) -> bool:
        return self._available

    # ------------------------------------------------------------------
    # 抽象接口
    # ------------------------------------------------------------------
    def embed_texts(self, texts: list[str]) -> list[list[float]]:
        """
        批量编码文本，返回 BGE-M3 dense 向量（1024 维，已 L2 归一化）。

        Args:
            texts: 文本列表。

        Returns:
            与 texts 等长的列表，第 i 项是 texts[i] 的 dense 向量；
            空文本 / 引擎不可用 / 编码失败时对应 `[]`，调用方据此跳过落库。
        """
        if not self._available or not texts:
            return [[] for _ in texts]

        # 过滤空文本，但保持索引对齐：用占位 [] 标记「跳过」
        encoded: list[list[float]] = [[] for _ in texts]
        valid_idx = [i for i, t in enumerate(texts) if t and t.strip()]
        valid_texts = [texts[i] for i in valid_idx]
        if not valid_texts:
            return encoded

        all_vectors: list[list[float]] = []
        try:
            # batch ≤ BGE_MAX_BATCH：长文本批量推理限批，避免内存尖峰
            for start in range(0, len(valid_texts), BGE_MAX_BATCH):
                chunk = valid_texts[start:start + BGE_MAX_BATCH]
                all_vectors.extend(self._run_batch(chunk))
        except Exception as e:
            print(f"⚠️ [BGE] text embedding failed: {e}")
            return encoded

        for i, vec in zip(valid_idx, all_vectors):
            if len(vec) == BGE_DENSE_DIM:
                encoded[i] = vec
        return encoded

    def _run_batch(self, texts: list[str]) -> list[list[float]]:
        """单批推理：tokenize → ONNX → (N, 1024)。"""
        inputs = self._tokenizer(
            texts,
            return_tensors="np",
            padding=True,
            truncation=True,
            max_length=BGE_MAX_TOKENS,
        )
        input_ids = inputs["input_ids"].astype(np.int64)
        attention_mask = inputs["attention_mask"].astype(np.int64)

        # 导出时按 [input_ids, attention_mask] 顺序声明，onnxruntime 保持该顺序
        id_name = self._session.get_inputs()[0].name
        mask_name = self._session.get_inputs()[1].name
        onnx_inputs = {id_name: input_ids, mask_name: attention_mask}
        raw = self._session.run(None, onnx_inputs)[0]

        arr = np.asarray(raw, dtype=np.float32)
        if arr.ndim == 1:
            arr = np.expand_dims(arr, axis=0)
        return arr.tolist()
