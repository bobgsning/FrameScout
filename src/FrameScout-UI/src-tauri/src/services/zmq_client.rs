// 负责通过 ZeroMQ 与 Python 端进行 Protobuf 请求与响应编解码。

use prost::Message;
use crate::constants::RPC_TIMEOUT_SEARCH_MS;
use crate::model_code::{EncodedFrame, OcrLine};
use crate::proto::framescout as proto;

/// 构造一个 EncodeRequest 的**唯一入口**。
///
/// 协议每加一个新字段，只需改这里，不必回到各个调用点补 `..Default` 式的散落默认值。
/// 需要非默认值的调用方用结构体更新语法覆盖即可，例如 MCP 侧：
/// `EncodeRequest { source: "mcp".to_string(), ..build_request(payload) }`
pub fn build_request(payload: proto::encode_request::Payload) -> proto::EncodeRequest {
    proto::EncodeRequest {
        task_id: "TASK_GLOBAL".to_string(),
        payload: Some(payload),
        // 0 = 不限制返回帧数（保留旧行为）
        max_results: 0,
        // 不显式指定时走 SigLIP 768，兼容旧 worker
        model: proto::EmbeddingModel::Siglip768 as i32,
        session_id: String::new(),
        // 命令来源：Rust 侧未来据此决定是否走白名单 / 沙箱
        source: "app".to_string(),
    }
}

pub fn request_vector(
    payload: proto::encode_request::Payload,
    timeout_ms: i32,
) -> Result<Vec<EncodedFrame>, String> {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::REQ).map_err(|e| e.to_string())?;
    socket.set_rcvtimeo(timeout_ms).map_err(|e| e.to_string())?;
    socket.connect("tcp://127.0.0.1:16666").map_err(|e| e.to_string())?;

    let req = build_request(payload);
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();

    socket.send(buf, 0).map_err(|e| e.to_string())?;
    let reply_raw = socket.recv_bytes(0).map_err(|e| e.to_string())?;

    let res = proto::EncodeResponse::decode(&reply_raw[..]).map_err(|e| e.to_string())?;
    match res.result {
        Some(proto::encode_response::Result::Success(s)) => Ok(s
            .frames
            .into_iter()
            .map(|f| EncodedFrame {
                path: f.file_path,
                timestamp: f.timestamp,
                vector: f.vector,
                ocr_text: f.ocr_text,
                index_time: f.index_time,
                ocr_lines: f
                    .ocr_lines
                    .into_iter()
                    .map(|l| OcrLine {
                        text: l.text,
                        bbox: (l.bbox_left, l.bbox_top, l.bbox_right, l.bbox_bottom),
                        score: l.score,
                        lang: l.lang,
                        timestamp_ms: l.timestamp_ms,
                        payload_json: l.payload_json,
                    })
                    .collect(),
                dense_vector: f.dense_vector,
            })
            .collect()),
        Some(proto::encode_response::Result::Error(e)) => Err(e.message),
        None => Err("Unknown response".to_string()),
    }
}

/// BGE-M3 文本查询：把 query 编码成 dense 向量（1024 维），用于文本语义检索。
///
/// 与视觉查询（SigLIP text encoder）是两条检索公路：这里显式把 `model` 设为
/// `BGE_M3_DENSE`，worker 据此走 `BgeEngine` 而非 `SiglipEngine`。
/// 返回 `frames[0].dense_vector`；文本引擎不可用（worker 未加载 BGE 模型）时为空。
pub fn request_text_dense(text: String, timeout_ms: i32) -> Result<Vec<f32>, String> {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::REQ).map_err(|e| e.to_string())?;
    socket.set_rcvtimeo(timeout_ms).map_err(|e| e.to_string())?;
    socket.connect("tcp://127.0.0.1:16666").map_err(|e| e.to_string())?;

    let req = proto::EncodeRequest {
        model: proto::EmbeddingModel::BgeM3Dense as i32,
        ..build_request(proto::encode_request::Payload::Text(text))
    };
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    socket.send(buf, 0).map_err(|e| e.to_string())?;
    let reply_raw = socket.recv_bytes(0).map_err(|e| e.to_string())?;

    let res = proto::EncodeResponse::decode(&reply_raw[..]).map_err(|e| e.to_string())?;
    match res.result {
        Some(proto::encode_response::Result::Success(s)) => Ok(s
            .frames
            .into_iter()
            .next()
            .map(|f| f.dense_vector)
            .unwrap_or_default()),
        Some(proto::encode_response::Result::Error(e)) => Err(e.message),
        None => Err("Unknown response".to_string()),
    }
}

/// 纯文本条目入库：把一段文本交给 worker 用 BGE-M3 编码，返回 `(entry_id, dense)`。
/// 文本通道的「写入侧」——与视觉索引是两条公路（这里显式 `model=BGE_M3_DENSE`）。
pub fn request_text_entry(text: String, source_uri: String) -> Result<(String, Vec<f32>), String> {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::REQ).map_err(|e| e.to_string())?;
    // 债单 A13/C7：rcvtimeo -1（无限等待）换 RPC_TIMEOUT_SEARCH_MS——纯文本入库
    // 卡住时快速失败，而非 UI 永久转圈只能杀进程。
    socket.set_rcvtimeo(RPC_TIMEOUT_SEARCH_MS).map_err(|e| e.to_string())?;
    socket.connect("tcp://127.0.0.1:16666").map_err(|e| e.to_string())?;

    let task = proto::TextTask {
        text,
        source_uri,
        metadata: std::collections::HashMap::new(),
    };
    let req = proto::EncodeRequest {
        model: proto::EmbeddingModel::BgeM3Dense as i32,
        ..build_request(proto::encode_request::Payload::TextTask(task))
    };
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();
    socket.send(buf, 0).map_err(|e| e.to_string())?;
    let reply_raw = socket.recv_bytes(0).map_err(|e| e.to_string())?;

    let res = proto::EncodeResponse::decode(&reply_raw[..]).map_err(|e| e.to_string())?;
    match res.result {
        Some(proto::encode_response::Result::Success(s)) => match s.text_entries.into_iter().next() {
            Some(e) => Ok((
                e.entry_id,
                e.embedding.map(|emb| emb.dense).unwrap_or_default(),
            )),
            None => Err("worker returned no text entry".to_string()),
        },
        Some(proto::encode_response::Result::Error(e)) => Err(e.message),
        None => Err("Unknown response".to_string()),
    }
}
