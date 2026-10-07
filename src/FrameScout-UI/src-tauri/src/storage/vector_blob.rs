// =========================================================================
//  向量 BLOB 编解码（v3.2.0 工程纪律 · 字节序约定）
// =========================================================================
//  ⚠️ 写死的约定，跨进程不得偏离：
//   `frame_vectors.vector_f32` 存的是**小端序（little-endian）平面 f32 字节布局**，
//   768 维即 3072 字节，无 header、无 padding、无维度信息。
//     - Rust 写入：`f32::to_le_bytes` 依次拼接；读取：`f32::from_le_bytes`
//     - 若将来改由 Python worker 直写，必须是 `struct.pack('<768f', v)`
//   字节序不一致是「零重构」里最容易踩的隐性坑——它不报错，只是检索结果全错，
//   因此必须在工程层面用这一处编解码函数收口，禁止任何地方手写 pack/unpack。

/// 把 f32 向量编码为小端序平面字节（长度 = vec.len() * 4）。
pub fn encode_f32_le(vector: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vector.len() * 4);
    for v in vector {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// 解码小端序平面字节为 f32 向量。字节数不是 4 的倍数时返回 None。
pub fn decode_f32_le(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    Some(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// 解码并校验维度：字节数必须等于 `expected_dim * 4`，否则视为无效（不静默返回
/// 长度不符的向量，避免下游拿错维度的数据去算相似度）。
pub fn decode_f32_le_checked(bytes: &[u8], expected_dim: usize) -> Option<Vec<f32>> {
    if bytes.len() != expected_dim * 4 {
        return None;
    }
    decode_f32_le(bytes)
}

#[cfg(test)]
mod tests {
    use super::{decode_f32_le, decode_f32_le_checked, encode_f32_le};

    #[test]
    fn roundtrip_preserves_values() {
        let v = vec![1.0f32, -2.5, 0.0, 3.14159];
        let bytes = encode_f32_le(&v);
        assert_eq!(bytes.len(), 16);
        assert_eq!(decode_f32_le(&bytes).unwrap(), v);
    }

    #[test]
    fn layout_is_little_endian() {
        // 1.0f32 的小端字节恰为 00 00 80 3F
        assert_eq!(encode_f32_le(&[1.0f32]), vec![0x00, 0x00, 0x80, 0x3F]);
    }

    #[test]
    fn rejects_misaligned_or_wrong_dimension() {
        assert!(decode_f32_le(&[0u8, 1, 2]).is_none());
        assert!(decode_f32_le_checked(&[0u8; 16], 768).is_none());
        assert!(decode_f32_le_checked(&[0u8; 3072], 768).is_some());
    }
}
