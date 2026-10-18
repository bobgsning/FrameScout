// 在独立的后台线程中轮询 Python AI 引擎的存活状态，并向前端发送事件。

use prost::Message;
use tauri::{AppHandle, Emitter};
use crate::error::{ec, ec2, STATUS_ENGINE_CONNECTING, STATUS_ENGINE_FAILED, STATUS_ENGINE_READY};
use crate::proto::framescout as proto;

/// 同步 PING 一次 worker，返回是否就绪。
fn ping_engine_once() -> bool {
    let context = zmq::Context::new();
    let check_socket = match context.socket(zmq::REQ) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if check_socket.connect("tcp://127.0.0.1:16666").is_err() {
        return false;
    }

    let _ = check_socket.set_rcvtimeo(3000);

    let ping_req = crate::services::zmq_client::build_request(
        proto::encode_request::Payload::Text("PING_ENGINE".to_string()),
    );
    let mut buf = Vec::new();
    ping_req.encode(&mut buf).unwrap();

    let success = match check_socket.send(buf, 0) {
        Ok(()) => match check_socket.recv_bytes(0) {
            Ok(reply) => {
                if let Ok(resp) = proto::EncodeResponse::decode(&reply[..]) {
                    resp.result.is_some()
                } else {
                    false
                }
            }
            Err(_) => false,
        },
        Err(_) => false,
    };

    drop(check_socket);
    success
}

/// 同步等待 worker 就绪（PING 轮询）。最多 `max_retries` 次，每次间隔 3 秒。
///
/// 供索引分段重启（见 `index_cmd.rs`）使用：重启 worker 后需等它重新加载模型。
/// 返回 `true` 表示在限制次数内就绪，`false` 表示超时。
pub fn wait_until_ready(max_retries: u32) -> bool {
    for _ in 0..max_retries {
        if ping_engine_once() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    false
}

pub fn wait_for_engine_and_notify(app_handle: AppHandle) {
    let mut retries: u32 = 0;
    const MAX_RETRIES: u32 = 30;

    loop {
        if ping_engine_once() {
            let _ = app_handle.emit(
                "engine-status",
                serde_json::json!({
                    "status": "ready",
                    "message": ec(STATUS_ENGINE_READY)
                }),
            );
            println!("✅ AI Worker is ready!");
            break;
        }

        retries += 1;
        let _ = app_handle.emit(
            "engine-status",
            serde_json::json!({
                "status": "connecting",
                "retry": retries,
                "max_retries": MAX_RETRIES,
                "message": ec2(STATUS_ENGINE_CONNECTING, retries, MAX_RETRIES)
            }),
        );

        if retries >= MAX_RETRIES {
            let _ = app_handle.emit(
                "engine-status",
                serde_json::json!({
                    "status": "error",
                    "message": ec(STATUS_ENGINE_FAILED)
                }),
            );
            panic!("AI Worker failed to become ready within {} seconds.", MAX_RETRIES * 3);
        }

        println!("⏳ Retrying in 3 seconds... (attempt {}/{})", retries, MAX_RETRIES);
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}
