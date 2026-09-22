"""
ONNX Runtime 工具：硬件执行提供器（Execution Provider）选择。

本项目为纯 Windows 发行版，固定使用 `onnxruntime-directml` 包：
  - DirectML（DmlExecutionProvider）基于 DX12，NVIDIA / AMD / Intel 的
    GPU 均可加速，用户无需安装 CUDA / cuDNN，分发体验最好。
  - CPU（CPUExecutionProvider）始终作为兜底。

⚠️ 两个关键认知（历史上踩过坑）：
1. `ort.get_available_providers()` 返回的是"当前 pip 包编译了哪些执行
   提供器"，而不是"这台机器有什么硬件"。装了 onnxruntime-directml 就
   永远不会出现 CUDAExecutionProvider；装了 CPU 版 onnxruntime 则连
   Dml 都没有——即使机器上有 GPU。
2. 不要同时安装多个 onnxruntime 发行包：它们都写入 onnxruntime/ 同一
   目录、互相覆盖，且卸载时按各自的 RECORD 删文件、极易删坏另一个包。
   本 venv 曾因 CPU 版与 DirectML 版共存，导致实际生效的是 CPU 版、
   GPU 加速静默失效（功能不受影响，只是慢）。
"""
import onnxruntime as ort


def create_onnx_session(onnx_path):
    """
    创建 ONNX Runtime InferenceSession，自动选择硬件执行提供器。

    检测依据为 get_available_providers()（即当前包的能力），
    优先级 CUDA → DirectML → CPU。在当前 DirectML 发行包下，
    CUDA 分支不会命中，实际为 DirectML GPU 加速 + CPU 兜底。
    """
    available_providers = ort.get_available_providers()
    providers = []

    # CUDA 分支：仅当换装 onnxruntime-gpu 包时才会命中（当前包不含此 EP）
    if "CUDAExecutionProvider" in available_providers:
        providers.append("CUDAExecutionProvider")
        print("🔥 NVIDIA CUDA acceleration enabled!")

    # DirectML 分支：当前发行包（onnxruntime-directml）的正常路径，
    # 在 Windows 上 NVIDIA / AMD / Intel 的 GPU 均可加速
    if "DmlExecutionProvider" in available_providers:
        providers.append("DmlExecutionProvider")
        print("🔥 DirectML GPU acceleration enabled! (NVIDIA / AMD / Intel)")

    # CPU 始终作为兜底：DirectML 初始化失败或个别算子不支持时自动回退
    providers.append("CPUExecutionProvider")

    sess_options = ort.SessionOptions()
    sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL

    session = ort.InferenceSession(onnx_path, sess_options, providers=providers)

    # 打印"实际生效"的 provider（session.get_providers 而非请求列表），
    # 避免"以为在用 GPU、实际静默落到 CPU"却毫无察觉
    print(f"   Active providers: {session.get_providers()}")
    return session
