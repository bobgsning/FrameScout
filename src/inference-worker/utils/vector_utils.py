"""
向量工具：保证 ONNX 输出统一为 (N, target_dim) 的 List[List[float]]。

SigLIP 2 base 输出为 768 维，本项目的向量库（FrameResult.vector）
与 Rust 端索引均按 768 维设计，因此这里做强制维度收敛。

⚠️ 跨进程字节序约定（v3.2.0 工程纪律，写死）：
   SQLite 的 `frame_vectors.vector_f32` BLOB 一律是
   **小端序（little-endian）平面 f32 字节布局**，768 维 = 3072 字节，
   无 header、无 padding、无维度信息。
     · Rust 侧：src-tauri/src/storage/vector_blob.rs（encode/decode）
     · Python 侧：本文件的 pack_f32_le / unpack_f32_le
   当前由 Rust 侧入库（Python 只经 protobuf 传 `repeated float`），
   但若将来改成 Python 直写 BLOB，必须走 pack_f32_le，禁止手写 struct.pack。
   字节序不一致不报错、只是检索结果全错，是最难查的隐性坑。
"""
import struct

import numpy as np


def format_vector_output(raw_vec, target_dim=768):
    """
    无论 ONNX 输出是 1D / 2D / 3D，强制收敛为 (N, target_dim)。

    处理规则（按输入维度）：
    - 2D (N, 768)     ：标准情况，直接通过（SigLIP pooled 输出，已实测正常）
    - 3D (N, P, 768)  ：对第 2 维（patch/token）取平均 → (N, 768)
    - 1D (768,)       ：补一个 batch 维 → (1, 768)

    ⚠️ 注意：3D 分支的 mean 是"兜底"逻辑。如果未来更换的 ONNX 模型
    导出的是完整序列隐藏态 (N, seq, 768)，对所有 token 取平均并不等价于
    官方的 pooled embedding（text 应取 EOS/pooled token，vision 应取 pooled
    输出），向量质量会系统性下降。当前模型输出即 pooled 的 (N, 768)，
    此分支不会被触发，仅作为形状异常时的保险。
    """
    arr = np.array(raw_vec, dtype=np.float32)

    if arr.ndim == 3:
        # 3D 情况：通常为 (N, patch, dim)，对 patch 维度取平均
        arr = arr.mean(axis=1)
    elif arr.ndim == 1:
        # 1D 情况：转为 (1, dim)
        arr = np.expand_dims(arr, axis=0)

    # 保险：如果最后一维不是 target_dim，尝试 reshape
    # （reshape 失败会抛异常，由上层 run() 的 except 捕获并返回 500）
    if arr.shape[-1] != target_dim:
        arr = arr.reshape(arr.shape[0], -1, target_dim).mean(axis=1)

    return arr.tolist()


def pack_f32_le(vector: list[float], dim: int = 768) -> bytes:
    """
    把一条 float 向量打包成「小端序平面 f32」字节，与 Rust 侧
    `vector_blob::encode_f32_le` 严格等价。

    `<{}f` 中的 `<` 即 little-endian；`dim` 用于校验长度，不符即抛错
    （宁可早失败，也不要静默写入维度错误的向量）。
    """
    if len(vector) != dim:
        raise ValueError(f"vector dim mismatch: expected {dim}, got {len(vector)}")
    return struct.pack(f"<{dim}f", *vector)


def unpack_f32_le(blob: bytes, dim: int = 768) -> list[float]:
    """pack_f32_le 的逆操作，与 Rust 侧 `vector_blob::decode_f32_le_checked` 等价。"""
    if len(blob) != dim * 4:
        raise ValueError(f"blob size mismatch: expected {dim * 4} bytes, got {len(blob)}")
    return list(struct.unpack(f"<{dim}f", blob))
