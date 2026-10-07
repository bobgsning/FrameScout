// OCR 行级结果的落库（检索表 `media_ocr_entries` 的唯一写入口）。
//
// 语义（与文档 5.4 对齐）：
//   - `ocr_text` 保留为拼接全文（兼容旧搜索管道），
//     **检索与定位以 media_ocr_entries 为真数据源**。
//   - `timestamp` 让同一 path（视频）下多帧 OCR 各自归属；图片恒为 0。
//   - 坐标归一化 0~1 存储，避免分辨率依赖；原始四角另存 payload_json。
//   - 数据先存、高亮后做：即便当前不做红框高亮，存下几乎零成本，未来直接有料。

use rusqlite::{params, Connection};

use crate::model_code::OcrLine;

/// 覆盖写入某文件的全部 OCR 行级结果（先删后插）。
///
/// 必须与「帧向量写入」处在同一事务里调用，保证 OCR 与向量要么一起落库、
/// 要么一起回滚，不出现「向量在但文本块丢了」或反之的半截状态。
pub fn replace_ocr_entries(conn: &Connection, path: &str, lines: &[OcrLine]) -> Result<(), String> {
    conn.execute("DELETE FROM media_ocr_entries WHERE path = ?1", params![path])
        .map_err(|e| format!("delete ocr entries failed: {}", e))?;

    for line in lines {
        let (l, t, r, b) = line.bbox;
        conn.execute(
            "INSERT INTO media_ocr_entries
                (path, timestamp, text_chunk, bbox_left, bbox_top, bbox_right, bbox_bottom, conf, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                path,
                line.timestamp_ms,
                line.text,
                l,
                t,
                r,
                b,
                line.score,
                line.payload_json
            ],
        )
        .map_err(|e| format!("insert ocr entry failed: {}", e))?;
    }
    Ok(())
}
