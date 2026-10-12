// 在独立的后台线程中轮询 Python AI 引擎的存活状态，并向前端发送事件。

use prost::Message;
use std::net::{SocketAddr, TcpStream};
use tauri::{AppHandle, Emitter};
use crate::proto::framescout as proto;

pub fn wait_for_engine_and_notify(app_handle: AppHandle) {
    let mut retries = 0;
    const MAX_RETRIES: u32 = 30;
    let endpoint: SocketAddr = "127.0.0.1:5555".parse().unwrap();

    loop {
        // A Nuitka one-file worker needs time to extract and load its models.
        // Creating and dropping ZeroMQ sockets while no server is listening can
        // abort inside libzmq's Windows poller, so probe the TCP port first.
        if TcpStream::connect_timeout(&endpoint, std::time::Duration::from_millis(500)).is_err() {
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
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
        }

        let context = zmq::Context::new();
        let check_socket = match context.socket(zmq::REQ) {
            Ok(s) => s,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(3));
                continue;
            }
        };

        // Do not leave a pending connection behind while the one-file worker
        // is still extracting and loading its models.
        let _ = check_socket.set_linger(0);
        let _ = check_socket.set_sndtimeo(3000);

        if check_socket.connect("tcp://127.0.0.1:5555").is_err() {
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
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
