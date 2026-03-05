#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def should_zip(target: str, bin_path: Path) -> bool:
    if bin_path.suffix.lower() == ".exe":
        return True
    t = target.lower()
    return ("windows" in t) or t.endswith("msvc")


def create_tar_gz(archive_path: Path, stage_dir: Path) -> None:
    with tarfile.open(archive_path, "w:gz") as tf:
        tf.add(stage_dir, arcname=stage_dir.name)


def create_zip(archive_path: Path, stage_dir: Path) -> None:
    with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for p in stage_dir.rglob("*"):
            if p.is_dir():
                continue
            zf.write(p, arcname=str(p.relative_to(stage_dir.parent)))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Package a loopforge release archive + sha256 file.")
    parser.add_argument("--version", required=True, help="Version string, e.g. v0.1.0")
    parser.add_argument("--target", required=True, help="Target triple or label, e.g. x86_64-unknown-linux-gnu")
    parser.add_argument(
        "--bin",
        required=True,
        help="Path to built loopforge binary (loopforge or loopforge.exe)",
    )
    parser.add_argument("--out-dir", default="dist", help="Output directory (default: dist)")
    parser.add_argument(
        "--include",
        action="append",
        default=[],
        help="Optional file to include in the archive (repeatable)",
    )
    args = parser.parse_args(argv)

    repo_root = Path.cwd()
    out_dir = (repo_root / args.out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    bin_path = (repo_root / args.bin).resolve()
    if not bin_path.exists():
        print(f"error: --bin does not exist: {bin_path}", file=sys.stderr)
        return 2

    if not bin_path.is_file():
        print(f"error: --bin is not a file: {bin_path}", file=sys.stderr)
        return 2

    base_name = f"loopforge-{args.version}-{args.target}"
    stage_dir = out_dir / base_name
    if stage_dir.exists():
        shutil.rmtree(stage_dir)
    stage_dir.mkdir(parents=True, exist_ok=True)

    # Copy binary (keep its original filename; on Windows it's usually loopforge.exe).
    dest_bin = stage_dir / bin_path.name
    shutil.copy2(bin_path, dest_bin)
    if dest_bin.suffix.lower() != ".exe":
        os.chmod(dest_bin, 0o755)

    # Default include set if present.
    default_includes = ["README.md", "LICENSE", "LICENSE.txt"]
    includes: list[Path] = []
    for rel in default_includes:
        p = (repo_root / rel)
        if p.exists() and p.is_file():
            includes.append(p)
    for user_inc in args.include:
        p = (repo_root / user_inc)
        if p.exists() and p.is_file():
            includes.append(p)

    seen: set[Path] = set()
    for p in includes:
        rp = p.resolve()
        if rp in seen:
            continue
        seen.add(rp)
        shutil.copy2(rp, stage_dir / p.name)

    if should_zip(args.target, bin_path):
        archive_path = out_dir / f"{base_name}.zip"
        create_zip(archive_path, stage_dir)
    else:
        archive_path = out_dir / f"{base_name}.tar.gz"
        create_tar_gz(archive_path, stage_dir)

    digest = sha256_file(archive_path)
    sha_path = archive_path.with_suffix(archive_path.suffix + ".sha256")
    sha_path.write_text(f"{digest}  {archive_path.name}\n", encoding="utf-8")

    print(str(archive_path))
    print(str(sha_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
