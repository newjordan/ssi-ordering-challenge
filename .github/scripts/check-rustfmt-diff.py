#!/usr/bin/env python3
"""Enforce rustfmt without expanding pre-existing formatting debt."""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path


def run(*args: str) -> str:
    return subprocess.run(
        args, check=True, text=True, stdout=subprocess.PIPE
    ).stdout


def rustfmt(source: str) -> str:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".rs") as candidate:
        candidate.write(source)
        candidate.flush()
        return run(
            "rustfmt",
            "--edition",
            "2021",
            "--emit",
            "stdout",
            "--config",
            "skip_children=true",
            candidate.name,
        )


def base_source(base: str, path: str) -> str | None:
    result = subprocess.run(
        ("git", "show", f"{base}:{path}"),
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    return result.stdout if result.returncode == 0 else None


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <base-revision>", file=sys.stderr)
        return 2
    base = sys.argv[1]
    paths = run(
        "git", "diff", "--name-only", "--diff-filter=ACMR", base, "--", "*.rs"
    ).splitlines()
    failures: list[str] = []
    skipped: list[str] = []
    for path in paths:
        before = base_source(base, path)
        if before is not None and rustfmt(before) != before:
            skipped.append(path)
            continue
        current = Path(path).read_text()
        if rustfmt(current) != current:
            failures.append(path)
    if skipped:
        print("rustfmt: preserved pre-existing debt in " + ", ".join(skipped))
    if failures:
        print("rustfmt check failed:", file=sys.stderr)
        print("\n".join(f"  {path}" for path in failures), file=sys.stderr)
        return 1
    print("rustfmt check passed for new and previously formatted Rust files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
