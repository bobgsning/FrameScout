"""
FrameScout — Offline AI Search Inference Worker
极简启动入口：只负责构造 InferenceServer 并启动主循环。
"""
import sys

from server import InferenceServer


def main():
    print("🚀 Starting FrameScout Inference Worker...")
    try:
        server = InferenceServer()
    except FileNotFoundError as e:
        # 最常见启动失败：SigLIP ONNX 模型缺失。
        # 用干净的错误信息代替裸 traceback（对齐旧版 main.py 的行为）
        print(f"\n❌ [FATAL ERROR] {e}")
        sys.exit(1)

    server.run()


if __name__ == "__main__":
    main()
