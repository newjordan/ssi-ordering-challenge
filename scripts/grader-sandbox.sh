#!/usr/bin/env bash
# Bubblewrap boundary for untrusted dependency build scripts and proc macros.
# The worker runtime uses the narrower per-invocation Rust abstraction in
# src/sandbox.rs; do not replace that with one shared wrapper around the parent.
set -euo pipefail

MODE="${1:-}"
if [[ "$MODE" != "build" && "$MODE" != "self-check" ]]; then
  echo "usage: $0 {build|self-check}" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Linux" ]]; then
  echo "grader-sandbox: bubblewrap grader mode requires Linux" >&2
  exit 1
fi
if [[ "$(id -u)" -eq 0 ]]; then
  echo "grader-sandbox: trusted parent must not run as root" >&2
  exit 1
fi
if ! command -v bwrap >/dev/null 2>&1; then
  echo "grader-sandbox: bubblewrap is required" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="$ROOT/target"
SYSROOT="$(rustc --print sysroot)"
mkdir -p "$TARGET"

for tool in "$SYSROOT/bin/cargo" "$SYSROOT/bin/rustc"; do
  if [[ ! -x "$tool" ]]; then
    echo "grader-sandbox: required tool is absent: $tool" >&2
    exit 1
  fi
done

# Start from an empty root. System binaries/libraries and the Rust toolchain are
# read-only; the candidate checkout is read-only except for target/. Build
# scripts and proc macros therefore cannot inspect HOME, runner temp files, the
# future eval corpus, or mutate source/configuration.
BWRAP=(
  bwrap
  --unshare-pid
  --unshare-net
  --unshare-ipc
  --unshare-uts
  --unshare-cgroup-try
  --new-session
  --die-with-parent
  --cap-drop ALL
  --clearenv
  --tmpfs /
  --proc /proc
  --dev /dev
  --tmpfs /tmp
  --ro-bind /usr /usr
)
for path in /bin /sbin /lib /lib64; do
  if [[ -e "$path" ]]; then
    BWRAP+=(--ro-bind "$path" "$path")
  fi
done
BWRAP+=(
  --ro-bind "$SYSROOT" /toolchain
  --ro-bind "$ROOT" /workspace
  --bind "$TARGET" /workspace/target
  --dir /tmp/home
  --dir /tmp/cargo
  --setenv PATH /toolchain/bin:/usr/bin:/bin
  --setenv HOME /tmp/home
  --setenv CARGO_HOME /tmp/cargo
  --setenv CARGO_TARGET_DIR /workspace/target
  --setenv CARGO_NET_OFFLINE true
  --setenv RUSTC /toolchain/bin/rustc
  --chdir /workspace
)

if [[ "$MODE" == "self-check" ]]; then
  "${BWRAP[@]}" -- /bin/bash -ceu '
    [[ "$(id -u)" -ne 0 ]]
    [[ ! -e "${RUNNER_TEMP:-/__unset_runner_temp}" ]]
    [[ ! -e /root ]]
    if touch /workspace/.sandbox-write-probe 2>/dev/null; then
      echo "grader-sandbox: checkout unexpectedly writable" >&2
      rm -f /workspace/.sandbox-write-probe
      exit 1
    fi
    touch /workspace/target/.sandbox-write-probe
    rm /workspace/target/.sandbox-write-probe
    touch /tmp/write-probe
    SSI_REQUIRE_NETWORK_BLOCK=1 /workspace/.github/scripts/assert-no-network.sh
  '
  echo "grader-sandbox: build boundary self-check passed"
  exit 0
fi

exec "${BWRAP[@]}" -- /toolchain/bin/cargo build --release --offline --locked
