// 负责通过 ZeroMQ 与 Python 端进行 Protobuf 请求与响应编解码。

use prost::Message;
use crate::proto::framescout as proto;

pub fn request_vector(
    payload: proto::encode_request::Payload,
    timeout_ms: i32,
) -> Result<Vec<(String, f32, Vec<f32>, String, f64)>, String> {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::REQ).map_err(|e| e.to_string())?;
    socket.set_rcvtimeo(timeout_ms).map_err(|e| e.to_string())?;
    socket.connect("tcp://127.0.0.1:5555").map_err(|e| e.to_string())?;

    let req = proto::EncodeRequest {
        task_id: "TASK_GLOBAL".to_string(),
        payload: Some(payload),
        single_file_ocr_config: None,
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
            .map(|f| (f.file_path, f.timestamp, f.vector, f.ocr_text, f.index_time))
            .collect()),
        Some(proto::encode_response::Result::Error(e)) => Err(e.message),
        None => Err("Unknown response".to_string()),
    }
}