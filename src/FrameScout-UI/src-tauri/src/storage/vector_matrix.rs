// 内存连续向量矩阵（768 维），包含点乘余弦检索与批量裁剪清理。

#[derive(Default, Clone)]
pub struct ImageMeta {
    pub path: String,
    pub timestamp: f32,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
}

#[derive(Default)]
pub struct FlatVectorMatrix {
    pub dim: usize,
    pub flat_vectors: Vec<f32>,
    pub metadata: Vec<ImageMeta>,
}

impl FlatVectorMatrix {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            flat_vectors: Vec::with_capacity(1000 * dim),
            metadata: Vec::with_capacity(1000),
        }
    }

    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    pub fn push(
        &mut self,
        path: String,
        timestamp: f32,
        vector: Vec<f32>,
        ocr_text: String,
        user_note: String,
        index_time: f64,
    ) {
        if vector.len() != self.dim {
            println!("⚠️ Dimension mismatch: expected {}, got {}", self.dim, vector.len());
            return;
        }
        self.flat_vectors.extend_from_slice(&vector);
        self.metadata.push(ImageMeta {
            path,
            timestamp,
            ocr_text,
            user_note,
            index_time,
        });
    }

    pub fn contains_path(&self, path: &str) -> bool {
        self.metadata.iter().any(|m| m.path == path)
    }

    /// O(N·d) 暴力检索：点积相似度。
    /// worker 在导出 ONNX 时已对向量做 L2 归一化，因此点积等价于余弦相似度。
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Vec<(usize, f32)> {
        if self.flat_vectors.is_empty() || query_vec.len() != self.dim {
            return Vec::new();
        }

        let mut scores: Vec<(usize, f32)> = self.flat_vectors
            .chunks_exact(self.dim)
            .enumerate()
            .map(|(idx, chunk)| {
                let score: f32 = query_vec.iter().zip(chunk.iter()).map(|(a, b)| a * b).sum();
                (idx, score)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn remove_by_paths(&mut self, paths_to_remove: &[String]) {
        let mut new_flat = Vec::with_capacity(self.flat_vectors.len());
        let mut new_meta = Vec::with_capacity(self.metadata.len());

        for (idx, meta) in self.metadata.drain(..).enumerate() {
            if !paths_to_remove.contains(&meta.path) {
                let start = idx * self.dim;
                let end = start + self.dim;
                new_flat.extend_from_slice(&self.flat_vectors[start..end]);
                new_meta.push(meta);
            }
        }

        self.flat_vectors = new_flat;
        self.metadata = new_meta;
    }
}