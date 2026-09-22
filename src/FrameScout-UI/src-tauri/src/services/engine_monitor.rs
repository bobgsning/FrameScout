// 在独立的后台线程中轮询 Python AI 引擎的存活状态，并向前端发送事件。

use prost::Message;
use tauri::{AppHandle, Emitter};
use crate::proto::framescout as proto;

pub fn wait_for_engine_and_notify(app_handle: AppHandle) {
    let context = zmq::Context::new();
    let mut retries = 0;
    const MAX_RETRIES: u32 = 50;

    loop {
        let check_socket = match context.socket(zmq::REQ) {
            Ok(s) => s,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(3));
                continue;
            }
        };

        if check_socket.connect("tcp://127.0.0.1:5555").is_err() {
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
        }

        let _ = check_socket.set_rcvtimeo(3000);

        let ping_req = proto::EncodeRequest {
            task_id: "PING_INIT".to_string(),
            payload: Some(proto::encode_request::Payload::Text("PING_ENGINE".to_string())),
            single_file_ocr_config: None,
        };
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

        if success {
            let _ = app_handle.emit(
                "engine-status",
                serde_json::json!({
                    "status": "ready",
                    "message": "AI Worker is ready!"
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
                "message": format!("Connecting... attempt {}/{}", retries, MAX_RETRIES)
            }),
        );

        if retries >= MAX_RETRIES {
            let _ = app_handle.emit(
                "engine-status",
                serde_json::json!({
                    "status": "error",
                    "message": "AI Worker failed to become ready. Please check models and restart."
                }),
            );
            panic!("AI Worker failed to become ready within {} seconds.", MAX_RETRIES * 3);
        }

        println!("⏳ Retrying in 3 seconds... (attempt {}/{})", retries, MAX_RETRIES);
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}