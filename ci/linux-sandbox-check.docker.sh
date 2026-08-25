#!/usr/bin/env bash
# Run ci/linux-sandbox-check.sh on a non-Linux dev host via a privileged Linux
# container. On a Linux CI runner, invoke ci/linux-sandbox-check.sh directly
# (see .github/workflows/linux-sandbox.yml) — no container needed.
#
# --privileged + --init: bubblewrap needs unprivileged user namespaces, and the
# container init reaps the watchdog's SIGKILL'd descendants (otherwise the
# grandchild-teardown test sees zombies). The repo must already be prepared
# (scripts/prepare-build.sh has produced vendor/ + generated manifests).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${SSI_LINUX_IMAGE:-rust:1}"

exec docker run --rm --privileged --init -v "$ROOT":/src:ro "$IMAGE" bash -c '
  set -euo pipefail
  apt-get update -qq >/dev/null 2>&1
  apt-get install -y -qq bubblewrap util-linux python3 curl ca-certificates >/dev/null 2>&1
  ARCH="$(uname -m)"; case "$ARCH" in aarch64) T=aarch64-unknown-linux-musl;; *) T=x86_64-unknown-linux-musl;; esac
  curl -sSL "https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-$T.tar.gz" -o /tmp/cd.tgz
  tar xzf /tmp/cd.tgz -C /tmp
  cp /tmp/cargo-deny-0.20.2-$T/cargo-deny /usr/local/bin/ && chmod +x /usr/local/bin/cargo-deny
  sysctl -w kernel.unprivileged_userns_clone=1 >/dev/null 2>&1 || true
  sysctl -w kernel.apparmor_restrict_unprivileged_userns=0 >/dev/null 2>&1 || true
  id -u tester >/dev/null 2>&1 || useradd -m -u 1000 tester
  mkdir -p /work
  tar cf - -C /src --exclude=./target --exclude=./target-slice4 --exclude=./.git . | (cd /work && tar xf -)
  chown -R tester:tester /work
  # Generate manifests + vendor the tree first (matches the CI job), so the
  # offline build in the check works from a freshly prepared tree rather than
  # any possibly-stale generated manifests copied from the host.
  exec runuser -u tester -- env HOME=/home/tester CARGO_HOME=/work/.cargohome PATH="/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin" \
    bash -c "cd /work && bash scripts/prepare-build.sh && bash ci/linux-sandbox-check.sh /work"
'
