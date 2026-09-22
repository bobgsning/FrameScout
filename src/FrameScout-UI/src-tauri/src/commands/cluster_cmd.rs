// 负责在内存矩阵中进行 两两点积（余弦等价）相似度视觉聚类。

use tauri::State;
use crate::models::{AppState, ClusterGroup};

#[tauri::command]
pub async fn cluster_similar_images(
    state: State<'_, AppState>,
    threshold: f32,
) -> Result<Vec<ClusterGroup>, String> {
    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Ok(Vec::new());
    }

    let dim = memory.dim;
    let num_items = memory.len();
    let mut visited = vec![false; num_items];
    let mut groups = Vec::new();

    for i in 0..num_items {
        if visited[i] {
            continue;
        }
        visited[i] = true;

        let vec_i = &memory.flat_vectors[i * dim..(i + 1) * dim];
        let mut members = vec![memory.metadata[i].path.clone()];

        for j in (i + 1)..num_items {
            if visited[j] {
                continue;
            }
            let vec_j = &memory.flat_vectors[j * dim..(j + 1) * dim];
            let similarity: f32 = vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

            if similarity >= threshold {
                visited[j] = true;
                members.push(memory.metadata[j].path.clone());
            }
        }

        if members.len() > 1 {
            groups.push(ClusterGroup {
                group_id: groups.len() + 1,
                representative_path: members[0].clone(),
                member_paths: members,
            });
        }
    }

    Ok(groups)
}