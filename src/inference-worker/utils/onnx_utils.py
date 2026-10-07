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

    **健壮性修复**：旧实现把 DML + CPU 同时塞进 providers 列表，
    onnxruntime 在 DML 初始化失败（如显存不足 8007000E）时可能长时间
    卡住而非快速回退 CPU，导致语义搜索请求一直不返回。
    新实现先单独尝试 GPU provider，失败则捕获异常、明确回退纯 CPU——
    保证模型加载绝不因 GPU 异常而卡死。
    """
    available_providers = ort.get_available_providers()

    sess_options = ort.SessionOptions()
    sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL

    # 先收集可用的 GPU provider
    gpu_providers = []
    if "CUDAExecutionProvider" in available_providers:
        gpu_providers.append("CUDAExecutionProvider")
    if "DmlExecutionProvider" in available_providers:
        gpu_providers.append("DmlExecutionProvider")

    # 尝试 GPU：失败（显存不足/驱动问题）则捕获异常，回退纯 CPU
    if gpu_providers:
        try:
            providers = gpu_providers + ["CPUExecutionProvider"]
            session = ort.InferenceSession(onnx_path, sess_options, providers=providers)
            print(f"   Active providers: {session.get_providers()}")
            return session
        except Exception as e:
            print(f"⚠️ GPU provider init failed ({e}), falling back to CPU...")

    # 纯 CPU 兜底（绝不因 GPU 异常而卡死/崩溃）
    session = ort.InferenceSession(onnx_path, sess_options, providers=["CPUExecutionProvider"])
    print(f"   Active providers (CPU fallback): {session.get_providers()}")
    return session
