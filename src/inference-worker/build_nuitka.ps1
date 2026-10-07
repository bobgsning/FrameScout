# =============================================================================
#  FrameScout AI Worker — Nuitka 打包脚本
#
#  用途：把 src/inference-worker 编译成原生 ai_worker.exe（替代 PyInstaller）。
#  收益：无 Temp 解压延时（毫秒级冷启动）、体积 Tree-shaking 瘦 30~50%、
#        更低的杀软误报率。零业务代码改动。
#
#  前置：
#    1) 已建好 .venv 并装齐依赖（含 onnxruntime-directml，且自检通过 DML）
#    2) 已安装 Nuitka：pip install nuitka zstandard
#    3) 已安装 Visual Studio Build Tools（含「使用 C++ 的桌面开发」工作负载）
#
#  用法（在 src\inference-worker 下打开 PowerShell）：
#    .\build_nuitka.ps1              # 发布版：standalone + onefile（慢，单文件）
#    .\build_nuitka.ps1 -Onedir      # 调试版：仅 standalone（快，便于排错）
#    .\build_nuitka.ps1 -SkipDeploy  # 只编译，不复制到 Tauri bin
#    .\build_nuitka.ps1 -NoTorch     # 剔除 torch/torchvision（瘦身；⚠️ 须先把 transformers 降到 4.x）
#    .\build_nuitka.ps1 -Onedir -NoTorch
# =============================================================================
param(
    [switch]$Onedir,
    [switch]$SkipDeploy,
    [switch]$NoTorch
)

$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " FrameScout AI Worker — Nuitka 打包" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# ------------------------------------------------------------------
# 0. 前置检查
# ------------------------------------------------------------------
if (-not $env:VIRTUAL_ENV) {
    Write-Host "⚠️ 未检测到激活的虚拟环境，尝试激活 .\.venv ..." -ForegroundColor Yellow
    if (Test-Path ".\.venv\Scripts\Activate.ps1") {
        . .\.venv\Scripts\Activate.ps1
    } else {
        throw "未找到 .venv，请先执行：python -m venv .venv"
    }
}
# 用 venv 内的绝对路径，避免受 PATH 顺序影响
$python = Join-Path $env:VIRTUAL_ENV "Scripts\python.exe"
if (-not (Test-Path $python)) {
    throw "未找到 venv 的 python：$python"
}
Write-Host "✔ 虚拟环境：$env:VIRTUAL_ENV" -ForegroundColor Green

if (-not (Get-Command nuitka -ErrorAction SilentlyContinue)) {
    throw "未找到 nuitka。请先安装：pip install nuitka zstandard"
}
Write-Host "✔ Nuitka：$((Get-Command nuitka).Source)" -ForegroundColor Green

# DirectML 后端自检：onnxruntime 必须能列出 DmlExecutionProvider。
# ⚠️ 必须同时检查 python 退出码：若 onnxruntime 包损坏（namespace package、
#    __init__.py 被互删等），get_available_providers 会抛 AttributeError，
#    stdout 为空且 $LASTEXITCODE 非零。旧脚本只判 $providers 是否匹配 DML，
#    会漏掉这类"包已损坏"的情形，误把空结果当"无 DML"继续打包。
Write-Host "⏳ 自检 onnxruntime 执行提供器 ..." -ForegroundColor Cyan
$providers = & $python -c "import onnxruntime as ort; print('|'.join(ort.get_available_providers()))"
$ortExit = $LASTEXITCODE
if ($ortExit -ne 0) {
    Write-Host "⚠️ onnxruntime 自检失败（python 退出码 $ortExit）。" -ForegroundColor Red
    Write-Host "   若上方出现 AttributeError: module 'onnxruntime' has no attribute" -ForegroundColor Red
    Write-Host "   'get_available_providers'，说明 onnxruntime 包已损坏（namespace package）。" -ForegroundColor Red
    Write-Host "   典型成因：rapidocr_onnxruntime 的依赖声明是 onnxruntime>=1.7.0，" -ForegroundColor Red
    Write-Host "   pip 会再拉一个标准 onnxruntime，与 onnxruntime-directml 同名冲突、" -ForegroundColor Red
    Write-Host "   互相删坏对方文件（详见 BUILD.md §1.2 与 requirements.txt 注释）。" -ForegroundColor Red
} elseif ($providers -notmatch "DmlExecutionProvider") {
    Write-Host "⚠️ 未检测到 DmlExecutionProvider！当前 venv 生效的可能是标准 onnxruntime。" -ForegroundColor Red
    Write-Host "   （import 成功但不含 DML 提供器，多半也是上述共存冲突所致。）" -ForegroundColor Red
}
if ($ortExit -ne 0 -or ($providers -notmatch "DmlExecutionProvider")) {
    Write-Host "   修复步骤：" -ForegroundColor Red
    Write-Host "     pip uninstall -y onnxruntime onnxruntime-directml rapidocr-onnxruntime" -ForegroundColor Red
    Write-Host "     # 手动删除 site-packages\onnxruntime 残骸目录（namespace 包会残留）" -ForegroundColor Red
    Write-Host "     pip install onnxruntime-directml>=1.24.4" -ForegroundColor Red
    Write-Host "     pip install --no-deps rapidocr-onnxruntime" -ForegroundColor Red
    throw "DirectML 后端不可用，中止打包。"
}
Write-Host "✔ 执行提供器：$providers" -ForegroundColor Green

# ------------------------------------------------------------------
# 1. 编译
# ------------------------------------------------------------------
$capiDir = Join-Path $env:VIRTUAL_ENV "Lib\site-packages\onnxruntime\capi"
if (-not (Test-Path $capiDir)) {
    throw "未找到 $capiDir，onnxruntime-directml 可能未正确安装。"
}
# 损坏的 onnxruntime 包：capi 目录还在，但里面的 dll/pyd 被两个包的 RECORD
# 互相删清空了。Nuitka --include-data-dir 时只会得到 "No data files in
# directory" 警告，运行时则直接缺二进制而崩。显式校验目录非空，尽早暴露。
$capiFiles = @(Get-ChildItem -Path $capiDir -File -ErrorAction SilentlyContinue)
if ($capiFiles.Count -eq 0) {
    throw "$capiDir 下无任何文件——onnxruntime 包已损坏（capi 二进制被互删）。请按上方指引修复环境。"
}

$outDir = "dist_nuitka"
$nuitkaArgs = @(
    "main.py",
    "--standalone",
    "--msvc=latest",
    "--enable-plugin=numpy",
    "--include-package-data=onnxruntime",
    "--include-package-data=zmq",
    "--include-package-data=transformers",
    "--include-package-data=rapidocr_onnxruntime",
    "--include-data-dir=$capiDir=onnxruntime/capi",
    "--nofollow-import-to=pytest",
    "--nofollow-import-to=unittest",
    "--nofollow-import-to=tkinter",
    "--nofollow-import-to=matplotlib",
    # transformers 的 ONNX 导出工具（transformers.onnx / convert_graph_to_onnx）在
    # 函数体内有裸 `import onnx / tensorflow / tf2onnx`（无 is_*_available 守卫），
    # 均为软依赖；worker 推理走 onnxruntime（DirectML），这三者运行时完全用不到。
    # 未安装时 Nuitka 会在 DLL 检测阶段报 "failed to locate package '<name>'" 的
    # FATAL，故显式排除（与 torch 同理）。
    "--nofollow-import-to=onnx",
    "--nofollow-import-to=tensorflow",
    "--nofollow-import-to=tf2onnx",
    # scipy 是旧 EasyOCR 时代的残留依赖（109MB，已被 RapidOCR 取代），worker 推理
    # 完全不用（业务代码 0 import）。但 transformers 的 DETR/maskformer/grounding-dino
    # 等 image-processing 模块有模块级 `import scipy`（软依赖，用于匈牙利匹配等），
    # 会让 Nuitka 跟着编译 scipy 的巨型 C 文件，把 MSVC 编译器堆空间打爆
    # （fatal error C1060/C1002）。故排除，产物也不会带上 scipy。
    "--nofollow-import-to=scipy",
    # sympy 是 onnxruntime-directml 的依赖，但只在 onnxruntime/tools/symbolic_shape_infer.py
    # （可选符号形状推断工具）里被 import，核心推理 InferenceSession 不依赖它。
    # sympy.polys.polyquinticconst 是巨型常量表，编译成 C 后 2.6 万行，同样会打爆
    # MSVC 堆空间（C1002）。worker 不用，故排除。
    "--nofollow-import-to=sympy",
    "--assume-yes-for-downloads",
    "--output-dir=$outDir",
    "--output-filename=ai_worker.exe",
    "--remove-output",
    # ⚠️ 关键：--nofollow-import-to 默认启用 deployment flag "excluded-module-usage"，
    # 运行时任何对被排除模块的 import/find_spec 都会抛
    # "Module 'onnx' was actively excluded from Nuitka compilation" 错误。
    # 但 transformers 启动时会用 find_spec() 探测一系列软依赖（torch/onnx/tensorflow 等），
    # 误触发该拦截导致 worker 崩溃。禁用此 flag，让 find_spec 正常返回 None
    # （transformers 判定"不可用"继续走），被排除模块仅是"不编进产物"而非"运行时禁止引用"。
    "--no-deployment-flag=excluded-module-usage"
)
if (-not $Onedir) {
    $nuitkaArgs += "--onefile"
}

# 剔除 torch / torchvision：本项目推理全走 ONNX(DirectML)，运行时不需要 torch。
# ⚠️ 前提：transformers 必须降到 4.x。transformers 5.x 的 SigLIP image processor
#    继承自 TorchvisionBackend，并被 @requires("torch","torchvision") 装饰，无 torch 时
#    AutoProcessor.from_pretrained(...) 会抛 OptionalDependencyNotAvailable。4.x 的 processor
#    是纯 numpy/PIL 实现，可安全排除。5.x 下使用本开关会在运行时崩溃。
if ($NoTorch) {
    $tfVer = & $python -c "import transformers; print(transformers.__version__)" 2>$null
    if ($tfVer -match '^5\.') {
        throw "检测到 transformers $tfVer（5.x）。5.x 的 SigLIP image processor 硬依赖 torch，使用 -NoTorch 会在运行时崩溃。请先把 transformers 降到 4.x（>=4.50，如：pip install 'transformers==4.57.6'），再使用 -NoTorch；或去掉 -NoTorch 保留 torch 一起打包。"
    }
    Write-Host "✔ 已启用 -NoTorch：将排除 torch / torchvision" -ForegroundColor Green
    $nuitkaArgs += "--nofollow-import-to=torch"
    $nuitkaArgs += "--nofollow-import-to=torchvision"
    $nuitkaArgs += "--nofollow-import-to=torchaudio"
}

# Windows 中文系统默认编码为 GBK。Nuitka 的 anti-bloat 插件会在编译时 exec
# torch.utils._config_module 等模块以提取配置变量；torch 读取自身模板文件时
# 用了系统默认编码，遇 UTF-8 字符（如 em dash —）即抛
# UnicodeDecodeError('gbk', ... 'illegal multibyte sequence')，并升级为
# FATAL: anti-bloat: Error, failed to evaluate variables for 'torch.utils._config_module'.
# 强制 Python 全局走 UTF-8（PEP 540）即可消除该 FATAL。
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"

Write-Host ""
Write-Host "🔨 开始 Nuitka 编译（首次 10~30 分钟属正常，勿中断）..." -ForegroundColor Cyan
Write-Host "   模式：$(if ($Onedir) { 'standalone (onedir)' } else { 'standalone + onefile' })"
# 用 venv 内 python -m nuitka 调用，而非 nuitka.cmd：.cmd 包装器会吞掉真实
# 退出码，导致编译 FATAL 后 $LASTEXITCODE 仍为 0，脚本误判"编译完成"继续走。
& $python -m nuitka @nuitkaArgs
if ($LASTEXITCODE -ne 0) {
    throw "Nuitka 编译失败（退出码 $LASTEXITCODE）。"
}
Write-Host "✔ 编译完成" -ForegroundColor Green

# ------------------------------------------------------------------
# 2. 定位输出并复制外部模型资源
# ------------------------------------------------------------------
if ($Onedir) {
    $exeDir = Join-Path $outDir "ai_worker.dist"
    $exePath = Join-Path $exeDir "ai_worker.exe"
    # Nuitka 的 onedir 目录名取入口脚本名（main.dist），exe 名取 --output-filename。
    # 依次尝试：ai_worker.dist\ai_worker.exe → main.dist\ai_worker.exe → main.dist\main.exe。
    if (-not (Test-Path $exePath)) {
        $exeDir = Join-Path $outDir "main.dist"
        $exePath = Join-Path $exeDir "ai_worker.exe"
        if (-not (Test-Path $exePath)) {
            $exePath = Join-Path $exeDir "main.exe"
        }
    }
} else {
    $exeDir = $outDir   # onefile：exe 直接落在 outDir，模型放同目录
    $exePath = Join-Path $outDir "ai_worker.exe"
    if (-not (Test-Path $exePath)) {
        $exePath = Join-Path $outDir "main.exe"
    }
}
if (-not (Test-Path $exePath)) {
    throw "未找到编译产物 ai_worker.exe（已找：$exePath）"
}

# 模型资源（siglip2-base）是外部资源，必须随 exe 一起分发：
# config.py::get_base_path() 在 frozen 下取 exe 所在目录，因此 models/ 与 exe 同级。
$modelsSrc = Join-Path $PSScriptRoot "models"
if (Test-Path $modelsSrc) {
    $modelsDst = Join-Path $exeDir "models"
    Write-Host "📦 复制模型资源 models\ → $modelsDst ..." -ForegroundColor Cyan
    if (Test-Path $modelsDst) { Remove-Item -Recurse -Force $modelsDst }
    Copy-Item -Recurse -Force $modelsSrc $modelsDst
} else {
    Write-Host "⚠️ 未找到 models\ 目录，跳过模型复制（请手动放置 siglip2-base）。" -ForegroundColor Yellow
}
Write-Host "✔ 产物：$exePath" -ForegroundColor Green

# ------------------------------------------------------------------
# 3. 部署到 Tauri bin（可选）
# ------------------------------------------------------------------
if (-not $SkipDeploy) {
    $tauriBin = [System.IO.Path]::GetFullPath(
        (Join-Path $PSScriptRoot "..\FrameScout-UI\src-tauri\bin\ai_worker")
    )
    Write-Host ""
    Write-Host "🚚 部署到 Tauri bin ..." -ForegroundColor Cyan
    Write-Host "   目标：$tauriBin"

    if (Test-Path $tauriBin) { Remove-Item -Recurse -Force $tauriBin }
    New-Item -ItemType Directory -Force -Path $tauriBin | Out-Null

    if ($Onedir) {
        # onedir：把 .dist 内容平铺进 bin\ai_worker（Tauri 期望 bin\ai_worker\ai_worker.exe）
        Get-ChildItem -Path $exeDir -Force | ForEach-Object {
            Copy-Item -Recurse -Force $_.FullName $tauriBin
        }
    } else {
        # onefile：单 exe + 同级 models 目录
        Copy-Item -Force $exePath $tauriBin
        if (Test-Path (Join-Path $exeDir "models")) {
            Copy-Item -Recurse -Force (Join-Path $exeDir "models") $tauriBin
        }
    }
    Write-Host "✔ 部署完成" -ForegroundColor Green
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " 打包流程结束。" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
