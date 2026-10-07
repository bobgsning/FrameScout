// 负责在内存矩阵中进行 视觉相似度聚类。
//
// P2-7 / 第三轮 A14-A15/E5 改造：
//   - 簇代表改 medoid（簇内平均相似度最高的成员），不再取 members[0]（常是歪的过渡帧）；
//   - 保留 size==1 的簇（旧实现丢弃，用户看不到「独特」的单帧）；
//   - 大库采样预筛：>5000 帧时，每帧只与随机采样的候选做比较，避免 O(N²) 假死；
//   - 快照 flat_vectors 后释放读锁：聚类计算期间不持 memory_db 读锁，扫描/回写可并行。
//
// 债单 B7/B8 修复：
//   - B7：旧采样取「紧随其后的连续 200 帧」（`.chunks(200).next()`），i+200 之后的同类帧
//     永远进不了这个簇 ⇒ 同一场景被切成大量碎片簇。现改为真随机采样
//     （xorshift 伪随机 + Fisher-Yates 部分洗牌，不引入 rand 依赖）。
//   - B8：旧 `members.len() > 1 || groups.len() < 50` 限制的是「总组数」，且被丢弃的
//     单帧簇静默消失。现单独计数「单帧簇 ≤50」，并在返回值中告知被截断的独特帧数。

use tauri::State;
use crate::model_code::{AppState, ClusterGroup, ClusterResult};
use crate::constants::CLUSTER_SAMPLE_THRESHOLD;

const CLUSTER_SAMPLE_SIZE: usize = 200; // 大库采样时每帧比较的候选数
const MAX_SINGLE_FRAME_CLUSTERS: usize = 50; // 单帧簇数量上限（债单 B8：限制单帧簇，而非总组数）

/// xorshift64 伪随机数生成器（债单 B7：真随机采样，避免引入 rand 依赖）。
struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        XorShift(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// 返回 [0, n) 均匀随机整数。
    fn gen_range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[tauri::command]
pub async fn cluster_similar_images(
    state: State<'_, AppState>,
    threshold: f32,
) -> Result<ClusterResult, String> {
    // 1) 快照 flat_vectors + metadata，然后释放读锁（P2-7 / C9）
    //    旧实现全程持读锁，聚类期间扫描/回写全部停摆。
    let (flat_vectors, dim, metadata) = {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Ok(ClusterResult {
                groups: Vec::new(),
                truncated_single_frames: 0,
            });
        }
        (memory.flat_vectors.clone(), memory.dim, memory.metadata.clone())
    };

    let num_items = metadata.len();
    let mut visited = vec![false; num_items];
    let mut groups: Vec<ClusterGroup> = Vec::new();
    let mut single_frame_clusters = 0usize;
    let mut truncated_single_frames = 0usize;

    // 2) 大库采样预筛（P2-7）
    let use_sampling = num_items > CLUSTER_SAMPLE_THRESHOLD;

    for i in 0..num_items {
        if visited[i] {
            continue;
        }
        visited[i] = true;

        let vec_i = &flat_vectors[i * dim..(i + 1) * dim];
        let mut members = vec![i];

        if use_sampling {
            // 债单 B7：真随机采样（xorshift + Fisher-Yates 部分洗牌），
            // 而非旧实现「取紧随其后的连续 200 帧」。
            let mut rng = XorShift::new(i as u64 ^ 0x9E37_79B9_7F4A_7C15);
            let mut unvisited: Vec<usize> = (i + 1..num_items)
                .filter(|j| !visited[*j])
                .collect();
            let take = CLUSTER_SAMPLE_SIZE.min(unvisited.len());
            for k in 0..take {
                let r = k + rng.gen_range(unvisited.len() - k);
                unvisited.swap(k, r);
            }
            unvisited.truncate(take);

            for &j in &unvisited {
                if visited[j] {
                    continue;
                }
                let vec_j = &flat_vectors[j * dim..(j + 1) * dim];
                let similarity: f32 = vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();
                if similarity >= threshold {
                    visited[j] = true;
                    members.push(j);
                }
            }
        } else {
            // 全量比较（小库）
            for j in (i + 1)..num_items {
                if visited[j] {
                    continue;
                }
                let vec_j = &flat_vectors[j * dim..(j + 1) * dim];
                let similarity: f32 = vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();
                if similarity >= threshold {
                    visited[j] = true;
                    members.push(j);
                }
            }
        }

        // 3) 簇代表改 medoid：计算簇内每帧与其它帧的平均相似度，取最高的
        //    旧实现取 members[0]（最先被遍历到的），常是歪的过渡帧。
        let representative_idx = if members.len() == 1 {
            members[0]
        } else {
            let mut best_idx = members[0];
            let mut best_avg = f32::MIN;
            for &m in &members {
                let vec_m = &flat_vectors[m * dim..(m + 1) * dim];
                let total: f32 = members
                    .iter()
                    .filter(|&&n| n != m)
                    .map(|&n| {
                        let vec_n = &flat_vectors[n * dim..(n + 1) * dim];
                        vec_m.iter().zip(vec_n.iter()).map(|(a, b)| a * b).sum::<f32>()
                    })
                    .sum::<f32>();
                let avg = total / (members.len() - 1) as f32;
                if avg > best_avg {
                    best_avg = avg;
                    best_idx = m;
                }
            }
            best_idx
        };

        // 4) 债单 B8：限制「单帧簇 ≤50」，而非「总组数 ≤50」。
        //    多帧簇总是保留；单帧簇超过上限时丢弃并计数（在返回值告知，不再静默消失）。
        if members.len() == 1 {
            if single_frame_clusters < MAX_SINGLE_FRAME_CLUSTERS {
                single_frame_clusters += 1;
            } else {
                truncated_single_frames += 1;
                continue;
            }
        }

        let member_paths: Vec<String> = members
            .iter()
            .map(|&idx| metadata[idx].path.clone())
            .collect();
        groups.push(ClusterGroup {
            group_id: groups.len() + 1,
            representative_path: metadata[representative_idx].path.clone(),
            member_paths,
        });
    }

    Ok(ClusterResult {
        groups,
        truncated_single_frames,
    })
}
