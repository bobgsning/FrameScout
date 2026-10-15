#!/usr/bin/env python3
"""
FrameScout 性能基准脚本（P2-12 / 第三轮「性能基准与回归测试」）。

目的：建立可重复的性能基线，让「换 Nuitka 有收益」等说法有数据支撑而非凭感觉。

用法：
    # 纯 SigLIP 2 视觉编码（不开 OCR）
    python scripts/benchmark.py --library <测试库路径>

    # SigLIP 2 + OCR（RapidOCR，en/ch_sim）
    python scripts/benchmark.py --library <测试库路径> --ocr

批次方式：默认按 batch_size=4 分批发送（对齐生产 index_cmd.rs 的 4 张一批），
          而非一次性塞入单个超大 batch（那会触发 worker 的资源问题，见 mydocs/benchmarks/README.md）。

模式：
    - 默认（不带 --ocr）：纯 SigLIP 2 视觉编码
    - 带 --ocr：SigLIP 2 视觉编码 + OCR + BGE-M3 文本向量

输出：benchmarks/YYYY-MM-DD_siglip.json 或 benchmarks/YYYY-MM-DD_ocr.json，含：
    - 索引：每批耗时、总耗时、吞吐、帧数、平均每文件耗时、失败批次数
    - 搜索延迟：P50 / P95 / mean / min / max

依赖：无需额外安装（标准库 + pyzmq + protobuf）。
注意：需先启动 worker（python main.py，监听 tcp://127.0.0.1:16666）。
"""
import argparse
import io
import json
import os
import sys
import time
import typing
from datetime import datetime, timezone

# 中文 Windows 下 stdout 默认 GBK，print emoji 会 UnicodeEncodeError。
# 与 config.py 相同处理：统一 stdout/stderr 为 UTF-8，避免运行时崩溃。
for _stream in (sys.stdout, sys.stderr):
    stream = typing.cast(io.TextIOWrapper, _stream)
    if hasattr(stream, "reconfigure"):
        stream.reconfigure(encoding="utf-8", errors="replace")

# 让从 scripts/ 子目录运行本脚本时，也能 import 到 worker 根目录的 framescout_pb2
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

MEDIA_EXTS = {'jpg', 'jpeg', 'png', 'webp', 'mp4', 'mov', 'avi', 'mkv'}
WORKER_ENDPOINT = "tcp://127.0.0.1:16666"
INDEX_TIMEOUT_MS = 1800000  # 30 分钟（冷启动 + 大规模编码）
SEARCH_TIMEOUT_MS = 10000   # 单次文本查询 10 秒


def _collect_files(library_path: str) -> list:
    """扫描测试库，返回可识别媒体文件的绝对路径列表（保持 os.walk 顺序）。"""
    file_paths = []
    for root, _, files in os.walk(library_path):
        for f in files:
            ext = f.lower().rsplit('.', 1)[-1] if '.' in f else ''
            if ext in MEDIA_EXTS:
                file_paths.append(os.path.join(root, f))
    return file_paths


def measure_indexing(library_path: str, batch_size: int = 4, enable_ocr: bool = False) -> dict:
    """按 batch_size 分批测量索引耗时（对齐生产 index_cmd.rs 的 4 张一批）。"""
    import zmq
    import framescout_pb2 as pb

    file_paths = _collect_files(library_path)
    file_count = len(file_paths)
    batch_total = (file_count + batch_size - 1) // batch_size
    print(f"  索引 {file_count} 个文件（batch_size={batch_size}, OCR={enable_ocr}）...")

    ctx = zmq.Context()
    socket = ctx.socket(zmq.REQ)
    socket.setsockopt(zmq.RCVTIMEO, INDEX_TIMEOUT_MS)
    socket.connect(WORKER_ENDPOINT)

    total_frames = 0
    per_batch_sec = []
    failed_batches = 0
    total_start = time.perf_counter()

    for batch_no in range(batch_total):
        start_idx = batch_no * batch_size
        chunk = file_paths[start_idx:start_idx + batch_size]

        req = pb.EncodeRequest()
        req.task_id = f"BENCH_{int(time.time())}_{batch_no}"
        req.source = "benchmark"
        req.session_id = "benchmark_session"
        req.batch.file_paths.extend(chunk)
        req.batch.ocr_config.enable_ocr = enable_ocr
        if enable_ocr:
            req.batch.ocr_config.languages.extend(["en", "ch_sim"])

        batch_start = time.perf_counter()
        socket.send(req.SerializeToString())
        resp_data = socket.recv()
        batch_elapsed = time.perf_counter() - batch_start
        per_batch_sec.append(round(batch_elapsed, 4))

        resp = pb.EncodeResponse()
        resp.ParseFromString(resp_data)
        if resp.HasField('success'):
            frames = len(resp.success.frames)
            total_frames += frames
            print(f"  批次 {batch_no + 1}/{batch_total}: {len(chunk)} 文件 → {frames} 帧, "
                  f"{batch_elapsed:.3f}s")
        else:
            failed_batches += 1
            msg = resp.error.message if resp.HasField('error') else 'unknown'
            print(f"  ⚠️ 批次 {batch_no + 1}/{batch_total} 失败: {msg}")

    total_elapsed = time.perf_counter() - total_start

    return {
        "file_count": file_count,
        "batch_size": batch_size,
        "enable_ocr": enable_ocr,
        "batch_count": batch_total,
        "failed_batches": failed_batches,
        "total_elapsed_sec": round(total_elapsed, 3),
        "per_batch_sec": per_batch_sec,
        "files_per_sec": round(file_count / total_elapsed, 3) if total_elapsed > 0 else 0,
        "frames_encoded": total_frames,
        "avg_ms_per_file": round(total_elapsed * 1000 / file_count, 1) if file_count > 0 else 0,
    }


def measure_search_latencies(queries: list, repeats: int = 10) -> list:
    """测量文本查询延迟。对每个查询跑 repeats 次取 P50/P95/mean/min/max。"""
    import zmq
    import framescout_pb2 as pb

    ctx = zmq.Context()
    socket = ctx.socket(zmq.REQ)
    socket.setsockopt(zmq.RCVTIMEO, SEARCH_TIMEOUT_MS)
    socket.connect(WORKER_ENDPOINT)

    results = []
    for query in queries:
        latencies = []
        hit_count = 0
        for _ in range(repeats):
            req = pb.EncodeRequest()
            req.task_id = f"BENCH_SEARCH_{int(time.time() * 1000)}"
            req.source = "benchmark"
            req.session_id = "benchmark_session"
            # 文本查询走 oneof 的 `text` 字段；model 默认 0 = SigLIP text encoder
            req.text = query

            start = time.perf_counter()
            socket.send(req.SerializeToString())
            resp_data = socket.recv()
            elapsed = (time.perf_counter() - start) * 1000  # ms

            resp = pb.EncodeResponse()
            resp.ParseFromString(resp_data)
            hit_count = len(resp.success.frames) if resp.HasField('success') else 0
            latencies.append(elapsed)

        latencies.sort()
        n = len(latencies)
        results.append({
            "query": query,
            "p50_ms": round(latencies[n // 2], 1),
            "p95_ms": round(latencies[min(int(n * 0.95), n - 1)], 1),
            "mean_ms": round(sum(latencies) / n, 1),
            "min_ms": round(min(latencies), 1),
            "max_ms": round(max(latencies), 1),
            "hit_count": hit_count,
        })

    return results


def main():
    parser = argparse.ArgumentParser(description="FrameScout 性能基准")
    parser.add_argument("--library", required=True, help="测试库路径")
    parser.add_argument("--output", default="benchmarks", help="输出目录")
    parser.add_argument("--batch-size", type=int, default=4, help="每批文件数（对齐生产 4 张一批）")
    parser.add_argument("--ocr", action="store_true", help="开启 OCR（默认纯 SigLIP 视觉编码）")
    parser.add_argument("--queries", nargs="*", default=["架构", "白板", "service mesh", "猫", "meeting"],
                        help="搜索查询列表")
    parser.add_argument("--repeats", type=int, default=10, help="每个查询重复次数")
    args = parser.parse_args()

    if not os.path.isdir(args.library):
        print(f"❌ 库路径不存在：{args.library}")
        sys.exit(1)

    os.makedirs(args.output, exist_ok=True)

    mode_label = "SigLIP2+OCR" if args.ocr else "SigLIP2"
    print("📊 FrameScout 性能基准")
    print(f"  模式：{mode_label}")
    print(f"  库路径：{args.library}")
    print(f"  batch_size：{args.batch_size}")
    print(f"  查询：{args.queries}")
    print()

    report = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "mode": "siglip_ocr" if args.ocr else "siglip",
        "library_path": args.library,
        "batch_size": args.batch_size,
        "enable_ocr": args.ocr,
        "platform": sys.platform,
        "python_version": sys.version,
    }

    # 1) 索引基准
    print("1️⃣  索引基准...")
    try:
        idx = measure_indexing(args.library, batch_size=args.batch_size, enable_ocr=args.ocr)
        report["indexing"] = idx
        print(f"  ✅ {idx['file_count']} 文件 / {idx['total_elapsed_sec']}s "
              f"({idx['files_per_sec']} files/s, {idx['avg_ms_per_file']} ms/文件, "
              f"{idx['frames_encoded']} 帧)")
    except Exception as e:
        report["indexing_error"] = str(e)
        print(f"  ❌ 索引失败：{e}")

    # 2) 搜索延迟基准
    print("\n2️⃣  搜索延迟基准...")
    try:
        report["search"] = measure_search_latencies(args.queries, repeats=args.repeats)
        for r in report["search"]:
            print(f"  '{r['query']}': P50={r['p50_ms']}ms P95={r['p95_ms']}ms hits={r['hit_count']}")
    except Exception as e:
        report["search_error"] = str(e)
        print(f"  ❌ 搜索失败：{e}")

    # 输出（文件名带模式后缀，纯 SigLIP 与 SigLIP+OCR 结果互不覆盖）
    today = datetime.now().strftime("%Y-%m-%d")
    mode_suffix = "ocr" if args.ocr else "siglip"
    output_file = os.path.join(args.output, f"{today}_{mode_suffix}.json")
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"\n✅ 基准报告已写入：{output_file}")


if __name__ == "__main__":
    main()
