import base64
import os
from cryptography.hazmat.primitives.asymmetric import ed25519

PRIVATE_KEY_FILE = "developer_private_key.pem"
PUBLIC_KEY_FILE = "developer_public_key.pem"

def generate_keypair_if_not_exists():
    """首次运行生成开发者的公私钥对"""
    if not os.path.exists(PRIVATE_KEY_FILE):
        private_key = ed25519.Ed25519PrivateKey.generate()
        public_key = private_key.public_key()

        # 保存私钥（极其重要！请妥善保管，绝对不能泄露）
        with open(PRIVATE_KEY_FILE, "wb") as f:
            f.write(private_key.private_bytes_raw())

        # 保存公钥（这个要复制到 Rust 代码中）
        with open(PUBLIC_KEY_FILE, "wb") as f:
            f.write(public_key.public_bytes_raw())

        print("🔑 New Ed25519 Keypair generated!")
        print(f"   Private Key saved to: {PRIVATE_KEY_FILE}")
        print(f"   Public Key (Hex): {public_key.public_bytes_raw().hex()}")
    else:
        print("🔑 Loaded existing keypair.")

def issue_license(user_email: str) -> str:
    """根据买家 Email 生成加密 License Key"""
    with open(PRIVATE_KEY_FILE, "rb") as f:
        private_key = ed25519.Ed25519PrivateKey.from_private_bytes(f.read())

    # 1. 构建 Payload (格式: Email|PRODUCT_NAME)
    payload_str = f"{user_email.strip().lower()}|FRAMESCOUT_PRO_V3"
    payload_bytes = payload_str.encode('utf-8')

    # 2. 使用私钥签名
    signature = private_key.sign(payload_bytes)

    # 3. 组合 Payload 和 签名并转为 Base64
    combined = payload_bytes + b"::" + signature
    license_key = base64.b64encode(combined).decode('utf-8')
    return license_key

if __name__ == "__main__":
    generate_keypair_if_not_exists()
    
    print("\n--------------------------------------------------")
    print("💎 FrameScout Pro - Offline License Generator")
    print("--------------------------------------------------")
    email = input("Enter buyer's Email address: ")
    if email:
        key = issue_license(email)
        print("\n✅ License Issued Successfully!")
        print(f"User Email:  {email}")
        print(f"License Key: {key}\n")