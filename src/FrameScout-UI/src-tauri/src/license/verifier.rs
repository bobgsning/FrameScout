// 负责 Pro 版本的 Ed25519 签名验证和本地 .lic 授权文件读取。

#[cfg(feature = "pro")]
use std::fs;
#[cfg(feature = "pro")]
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
#[cfg(feature = "pro")]
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[cfg(feature = "pro")]
pub const DEVELOPER_PUBLIC_KEY_HEX: &str = "4e5707fcc7233e50254479f5defb781a6ba0474c0b087809a234bfba9368316c";
#[cfg(feature = "pro")]
pub const PRODUCT_TAG: &str = "FRAMESCOUT_PRO_V3";

#[cfg(feature = "pro")]
pub fn verify_license_key(email: &str, license_key: &str) -> Result<(), String> {
    let pub_key_bytes = hex::decode(DEVELOPER_PUBLIC_KEY_HEX)
        .map_err(|e| format!("Failed to decode public key hex: {}", e))?;
    let verifying_key = VerifyingKey::try_from(pub_key_bytes.as_slice())
        .map_err(|e| format!("Failed to parse public key: {}", e))?;

    let decoded_bytes = BASE64.decode(license_key.trim())
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    // Ed25519 签名为固定 64 字节。由于签名是二进制数据，内部可能偶然出现
    // 字节序列 b"::"，若从前往后查找首个分隔符会解析错位，导致合法授权被误判。
    // 因此从末尾反推分隔符位置：总长度 - 64(签名) - 2(分隔符)。
    if decoded_bytes.len() < 66 {
        return Err("License key too short".to_string());
    }
    let sep_pos = decoded_bytes.len() - 64 - 2;
    if &decoded_bytes[sep_pos..sep_pos + 2] != b"::" {
        return Err("Malformed license key (missing '::' separator)".to_string());
    }

    let payload_bytes = &decoded_bytes[..sep_pos];
    let sig_bytes = &decoded_bytes[sep_pos + 2..];

    if sig_bytes.len() != 64 {
        return Err(format!("Invalid signature length: expected 64, got {}", sig_bytes.len()));
    }

    let expected_payload = format!("{}|{}", email.trim().to_lowercase(), PRODUCT_TAG);
    let expected_bytes = expected_payload.as_bytes();
    if payload_bytes != expected_bytes {
        return Err("Payload mismatch: email or product tag incorrect".to_string());
    }

    let signature = Signature::from_slice(sig_bytes)
        .map_err(|e| format!("Invalid signature bytes: {}", e))?;
    verifying_key.verify(payload_bytes, &signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;

    Ok(())
}

pub fn check_local_license() -> (bool, String) {
    #[cfg(feature = "pro")]
    {
        let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout-Offline_AI_Search-Global");
        let lic_path = app_data_dir.join("framescout.lic");

        match fs::read_to_string(lic_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 2 {
                    let email = lines[0].trim();
                    let key = lines[1].trim();
                    match verify_license_key(email, key) {
                        Ok(()) => {
                            println!("💎 Pro License Verified for: {}", email);
                            (true, email.to_string())
                        }
                        Err(e) => {
                            println!("⚠️ License verification failed: {}", e);
                            (false, "".to_string())
                        }
                    }
                } else {
                    (false, "".to_string())
                }
            }
            Err(_) => (false, "".to_string()),
        }
    }

    #[cfg(not(feature = "pro"))]
    {
        (false, "".to_string())
    }
}