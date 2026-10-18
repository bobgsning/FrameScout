// 负责 Pro 版本的 Ed25519 签名验证和本地 .lic 授权文件读取。

#[cfg(feature = "pro")]
use std::fs;
#[cfg(feature = "pro")]
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
#[cfg(feature = "pro")]
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
#[cfg(feature = "pro")]
use crate::error::{
    ec, ec1, ec2, ERR_LICENSE_BASE64_FAILED, ERR_LICENSE_CLOCK_ERROR, ERR_LICENSE_EMAIL_MISMATCH,
    ERR_LICENSE_EXPIRED, ERR_LICENSE_EXPIRY_INVALID, ERR_LICENSE_KEY_MALFORMED,
    ERR_LICENSE_KEY_TOO_SHORT, ERR_LICENSE_PAYLOAD_FIELDS, ERR_LICENSE_PAYLOAD_NOT_UTF8,
    ERR_LICENSE_PRODUCT_MISMATCH, ERR_LICENSE_PUBKEY_HEX_FAILED, ERR_LICENSE_PUBKEY_PARSE_FAILED,
    ERR_LICENSE_SIG_BYTES_FAILED, ERR_LICENSE_SIG_LENGTH, ERR_LICENSE_SIG_VERIFY_FAILED,
    ERR_LICENSE_VERSION_LOCKED,
};

#[cfg(feature = "pro")]
pub const DEVELOPER_PUBLIC_KEY_HEX: &str = "4e5707fcc7233e50254479f5defb781a6ba0474c0b087809a234bfba9368316c";
#[cfg(feature = "pro")]
pub const PRODUCT_TAG: &str = "FRAMESCOUT_PRO_V3";
/// 锁定版本：签发的 Key 只能在同版本应用上激活。取 Cargo.toml 的 `version`。
/// 升版本时改 Cargo.toml 的 version 后重新签发即可（旧 Key 因版本不匹配自动失效）。
#[cfg(feature = "pro")]
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "pro")]
pub fn verify_license_key(email: &str, license_key: &str) -> Result<(), String> {
    let pub_key_bytes = hex::decode(DEVELOPER_PUBLIC_KEY_HEX)
        .map_err(|e| ec1(ERR_LICENSE_PUBKEY_HEX_FAILED, e))?;
    let verifying_key = VerifyingKey::try_from(pub_key_bytes.as_slice())
        .map_err(|e| ec1(ERR_LICENSE_PUBKEY_PARSE_FAILED, e))?;

    let decoded_bytes = BASE64.decode(license_key.trim())
        .map_err(|e| ec1(ERR_LICENSE_BASE64_FAILED, e))?;

    // Ed25519 签名为固定 64 字节。由于签名是二进制数据，内部可能偶然出现
    // 字节序列 b"::"，若从前往后查找首个分隔符会解析错位，导致合法授权被误判。
    // 因此从末尾反推分隔符位置：总长度 - 64(签名) - 2(分隔符)。
    if decoded_bytes.len() < 66 {
        return Err(ec(ERR_LICENSE_KEY_TOO_SHORT));
    }
    let sep_pos = decoded_bytes.len() - 64 - 2;
    if &decoded_bytes[sep_pos..sep_pos + 2] != b"::" {
        return Err(ec(ERR_LICENSE_KEY_MALFORMED));
    }

    let payload_bytes = &decoded_bytes[..sep_pos];
    let sig_bytes = &decoded_bytes[sep_pos + 2..];

    if sig_bytes.len() != 64 {
        return Err(ec1(ERR_LICENSE_SIG_LENGTH, sig_bytes.len()));
    }

    // 1) 先验签：签名覆盖整个 payload，验签通过才说明 payload 未被篡改。
    //    （顺序很重要——必须先验签、再解析字段；否则攻击者可改 expiry/version 等字段。）
    let signature = Signature::from_slice(sig_bytes)
        .map_err(|e| ec1(ERR_LICENSE_SIG_BYTES_FAILED, e))?;
    verifying_key.verify(payload_bytes, &signature)
        .map_err(|e| ec1(ERR_LICENSE_SIG_VERIFY_FAILED, e))?;

    // 2) 验签通过后，解析 payload 字段。格式有两种（向后兼容）：
    //    新格式（4 段）email|PRODUCT|version|expiry_ts —— 校验版本锁定 + 过期；
    //    旧格式（2 段）email|PRODUCT —— 无版本/过期约束，继续有效。
    let payload_str = std::str::from_utf8(payload_bytes)
        .map_err(|e| ec1(ERR_LICENSE_PAYLOAD_NOT_UTF8, e))?;
    let fields: Vec<&str> = payload_str.split('|').collect();

    match fields.len() {
        4 => {
            // 3) email 匹配
            if fields[0] != email.trim().to_lowercase() {
                return Err(ec(ERR_LICENSE_EMAIL_MISMATCH));
            }
            // 4) 产品标识匹配
            if fields[1] != PRODUCT_TAG {
                return Err(ec(ERR_LICENSE_PRODUCT_MISMATCH));
            }
            // 5) 版本锁定
            if fields[2] != PRODUCT_VERSION {
                return Err(ec2(
                    ERR_LICENSE_VERSION_LOCKED,
                    fields[2],
                    PRODUCT_VERSION,
                ));
            }
            // 6) 过期检查（签发时 Python 写进 expiry_ts，这里与当前时间比对）
            let expiry_ts: i64 = fields[3]
                .parse()
                .map_err(|_| ec(ERR_LICENSE_EXPIRY_INVALID))?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| ec1(ERR_LICENSE_CLOCK_ERROR, e))?
                .as_secs() as i64;
            if now > expiry_ts {
                return Err(ec(ERR_LICENSE_EXPIRED));
            }
        }
        2 => {
            // 旧格式（向后兼容）：只校验 email + 产品标识，不校验版本/过期。
            if fields[0] != email.trim().to_lowercase() {
                return Err(ec(ERR_LICENSE_EMAIL_MISMATCH));
            }
            if fields[1] != PRODUCT_TAG {
                return Err(ec(ERR_LICENSE_PRODUCT_MISMATCH));
            }
        }
        n => {
            return Err(ec1(ERR_LICENSE_PAYLOAD_FIELDS, n));
        }
    }

    Ok(())
}

/// 从 License Key 解析「到期时间戳 + 锁定版本」（仅新格式 4 段 payload）。
/// 旧格式（3 段）返回 `None`。仅作展示用途（`get_license_status` 显示到期日），
/// 不验签——验签已由 `verify_license_key` 完成。
#[cfg(feature = "pro")]
pub fn parse_license_info(license_key: &str) -> Option<(i64, String)> {
    let decoded = BASE64.decode(license_key.trim()).ok()?;
    if decoded.len() < 66 {
        return None;
    }
    let sep_pos = decoded.len() - 64 - 2;
    if &decoded[sep_pos..sep_pos + 2] != b"::" {
        return None;
    }
    let payload = std::str::from_utf8(&decoded[..sep_pos]).ok()?;
    let fields: Vec<&str> = payload.split('|').collect();
    if fields.len() != 4 {
        return None;
    }
    let expiry: i64 = fields[3].parse().ok()?;
    Some((expiry, fields[2].to_string()))
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