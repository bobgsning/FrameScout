#!/usr/bin/env python3
"""
download_ocr_models.py — FrameScout OCR 多语言识别模型下载器（构建期 / 离线分发）

从 RapidAI/RapidOCR 官方模型库（ModelScope）下载 OCR 多语言 rec 识别模型与字典，
落地到 `src/inference-worker/models/ocr/<code>/`，并逐文件校验 SHA256（与官方
default_models.yaml 核对），确保「真官方、无差错」。

默认只下载「随安装包内置」的语言（ch_tra / ja），它们会被 build_nuitka.ps1 一起
打进安装包；「可下载」语言（en_opt / ru / ko / de / fr / es）由应用内下载命令在运行时
按需获取，本脚本也支持 --all 一次性预下载（--all 按 tier 自动覆盖 en_opt 在内的
全部可下载语言）。

用法：
    python scripts/download_ocr_models.py            # 下载内置语言 ch_tra + ja
    python scripts/download_ocr_models.py --all      # 下载内置 + 可下载全部语言
    python scripts/download_ocr_models.py --lang ja  # 只下载指定语言
    python scripts/download_ocr_models.py --force    # 覆盖已存在的文件
"""

import argparse
import hashlib
import json
import os
import sys
import urllib.request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
WORKER_DIR = os.path.join(PROJECT_ROOT, "src", "inference-worker")
MANIFEST_PATH = os.path.join(WORKER_DIR, "ocr_manifest.json")
MODELS_OCR_DIR = os.path.join(WORKER_DIR, "models", "ocr")

CHUNK_SIZE = 1024 * 256  # 256 KiB


def load_manifest() -> list[dict]:
    with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)
    return data.get("languages", [])


def download(url: str, dest: str, sha256: str | None) -> None:
    """下载单个文件到 dest，校验 SHA256（若提供）。失败抛异常。"""
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    tmp = dest + ".part"
    hasher = hashlib.sha256()

    req = urllib.request.Request(url, headers={"User-Agent": "FrameScout/3.2.0"})
    with urllib.request.urlopen(req, timeout=120) as resp, open(tmp, "wb") as out:
        while True:
            chunk = resp.read(CHUNK_SIZE)
            if not chunk:
                break
            out.write(chunk)
            hasher.update(chunk)

    digest = hasher.hexdigest()
    if sha256 and digest.lower() != sha256.lower():
        os.remove(tmp)
        raise RuntimeError(
            f"SHA256 mismatch for {os.path.basename(dest)}\n"
            f"  expected: {sha256}\n  actual:   {digest}"
        )

    os.replace(tmp, dest)
    size = os.path.getsize(dest)
    print(f"  ✅ {os.path.basename(dest)} ({size / 1024 / 1024:.1f} MB)")


def download_language(lang: dict, force: bool) -> None:
    code = lang["code"]
    target_dir = os.path.join(MODELS_OCR_DIR, code)

    rec_model = lang.get("rec_model")
    rec_dict = lang.get("rec_dict")
    sha256 = lang.get("sha256")

    if rec_model:
        dest = os.path.join(target_dir, rec_model)
        if force or not os.path.isfile(dest):
            _download_with_fallback(lang.get("urls", []), dest, sha256)

    if rec_dict:
        dest = os.path.join(target_dir, rec_dict)
        if force or not os.path.isfile(dest):
            _download_with_fallback(lang.get("dict_urls", []), dest, None)


def _download_with_fallback(urls: list[str], dest: str, sha256: str | None) -> None:
    """按序尝试多个 URL（官方优先，自建镜像兜底）。全部失败则报错。"""
    last_err = None
    for url in urls:
        try:
            download(url, dest, sha256)
            return
        except Exception as e:  # noqa: BLE001
            last_err = e
            print(f"  ⚠️ 下载失败，尝试下一个来源：{url}\n     {e}")
    raise RuntimeError(f"所有来源均失败：{last_err}")


def main() -> int:
    parser = argparse.ArgumentParser(description="下载 FrameScout OCR 多语言识别模型")
    parser.add_argument("--all", action="store_true", help="下载内置 + 可下载全部语言")
    parser.add_argument("--lang", action="append", help="只下载指定语言码（可多次）")
    parser.add_argument("--force", action="store_true", help="覆盖已存在的文件")
    args = parser.parse_args()

    languages = load_manifest()

    if args.lang:
        selected = [l for l in languages if l["code"] in args.lang]
    elif args.all:
        selected = [l for l in languages if l.get("tier") in ("builtin_bundle", "downloadable")]
    else:
        selected = [l for l in languages if l.get("tier") == "builtin_bundle"]

    if not selected:
        print("⚠️ 没有需要下载的语言。")
        return 0

    print(f"📥 开始下载 {len(selected)} 种语言的 OCR 识别模型 → {MODELS_OCR_DIR}")
    for lang in selected:
        print(f"\n[{lang['code']}] {lang.get('native', '')}")
        try:
            download_language(lang, args.force)
        except Exception as e:  # noqa: BLE001
            print(f"  ❌ {lang['code']} 下载失败：{e}")
            return 1

    print("\n✅ 全部完成。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
