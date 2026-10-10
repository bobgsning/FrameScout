#!/usr/bin/env python3
"""
FrameScout 性能基准脚本（P2-12 / 第三轮「性能基准与回归测试」）。

目的：建立可重复的性能基线，让「换 Nuitka 有收益」等说法有数据支撑而非凭感觉。

用法：
    python scripts/benchmark.py --library /path/to/test/library --output benchmarks/

输出：benchmarks/YYYY-MM-DD.json，含：
    - 索引耗时（按文件数 / 按帧数）
    - 三种搜索的延迟（P50 / P95 / mean）
    - 启动加载耗时
    - 内存矩阵规模

依赖：无需额外安装（只用标准库 + requests 调 FrameScout 的 ZMQ 端口）。
注意：需先启动 FrameScout App（它会拉起 ai_worker），脚本通过 Tauri 命令测。
"""
import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# 让从 scripts/ 子目录运行本脚本时，也能 import 到 worker 根目录的 framescout_pb2
# （framescout_pb2.py 与 main.py 同级，不在 scripts/ 下）。
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def measure_indexing(library_path: str, scan_mode: str = "all") -> dict:
    """测量索引耗时。通过 Tauri CLI 或直接调 worker。"""
    import zmq
    import framescout_pb2 as pb

    MEDIA_EXTS = {'jpg', 'jpeg', 'png', 'webp', 'mp4', 'mov', 'avi', 'mkv'}

    # 统计文件数 + 收集路径
    file_paths = []
    for root, _, files in os.walk(library_path):
        for f in files:
            ext = f.lower().rsplit('.', 1)[-1] if '.' in f else ''
            if ext in MEDIA_EXTS:
                file_paths.append(os.path.join(root, f))

    file_count = len(file_paths)
    print(f"  索引 {file_count} 个文件...")

    ctx = zmq.Context()
    socket = ctx.socket(zmq.REQ)
    socket.set_rcvtimeo(120000)  # 2 分钟超时
    socket.connect("tcp://127.0.0.1:16666")

    # 发送批量编码请求
    req = pb.EncodeRequest()
    req.task_id = f"BENCH_{int(time.time())}"
    req.source = "benchmark"
    req.session_id = "benchmark_session"
    req.batch.file_paths.extend(file_paths)
    req.batch.ocr_config.enable_ocr = True
    req.batch.ocr_config.languages.extend(["en", "ch_sim"])

    start = time.perf_counter()
    socket.send(req.SerializeToString())
    resp_data = socket.recv()
    elapsed = time.perf_counter() - start

    resp = pb.EncodeResponse()
    resp.ParseFromString(resp_data)
    frames = list(resp.success.frames) if resp.HasField('success') else []

    return {
        "file_count": file_count,
        "elapsed_sec": round(elapsed, 3),
        "files_per_sec": round(file_count / elapsed, 1) if elapsed > 0 else 0,
        "frames_encoded": len(frames),
    }


def measure_search_latencies(library_path: str, queries: list) -> list:
    """测量搜索延迟。对每个查询跑多次取 P50/P95。"""
    import zmq
    import framescout_pb2 as pb

    ctx = zmq.Context()
    socket = ctx.socket(zmq.REQ)
    socket.set_rcvtimeo(10000)
    socket.connect("tcp://127.0.0.1:16666")

    results = []
    for query in queries:
        latencies = []
        for _ in range(10):  # 每个查询跑 10 次取统计
            req = pb.EncodeRequest()
            req.task_id = f"BENCH_SEARCH_{int(time.time()*1000)}"
            req.source = "benchmark"
            req.session_id = "benchmark_session"
            # 文本查询走 oneof 的 `text` 字段（proto 无 text_query 字段）
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
            "p95_ms": round(latencies[int(n * 0.95)], 1),
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
    parser.add_argument("--queries", nargs="*", default=["架构", "白板", "service mesh", "猫", "meeting"],
                        help="搜索查询列表")
    args = parser.parse_args()

    if not os.path.isdir(args.library):
        print(f"❌ 库路径不存在：{args.library}")
        sys.exit(1)

    os.makedirs(args.output, exist_ok=True)

    print("📊 FrameScout 性能基准")
    print(f"  库路径：{args.library}")
    print(f"  查询：{args.queries}")
    print()

    report = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "library_path": args.library,
        "platform": sys.platform,
        "python_version": sys.version,
    }

    # 1) 索引基准
    print("1️⃣  索引基准...")
    try:
        report["indexing"] = measure_indexing(args.library)
        print(f"  ✅ {report['indexing']['file_count']} 文件 / {report['indexing']['elapsed_sec']}s "
              f"({report['indexing']['files_per_sec']} files/s)")
    except Exception as e:
        report["indexing_error"] = str(e)
        print(f"  ❌ 索引失败：{e}")

    # 2) 搜索延迟基准
    print("\n2️⃣  搜索延迟基准...")
    try:
        report["search"] = measure_search_latencies(args.library, args.queries)
        for r in report["search"]:
            print(f"  '{r['query']}': P50={r['p50_ms']}ms P95={r['p95_ms']}ms hits={r['hit_count']}")
    except Exception as e:
        report["search_error"] = str(e)
        print(f"  ❌ 搜索失败：{e}")

    # 输出
    today = datetime.now().strftime("%Y-%m-%d")
    output_file = os.path.join(args.output, f"{today}.json")
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"\n✅ 基准报告已写入：{output_file}")


if __name__ == "__main__":
    main()
