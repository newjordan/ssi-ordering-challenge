# Fix #47 — Linux runtime validation

Follow-up to [`FIX-47-local-sandbox.md`](FIX-47-local-sandbox.md). The macOS
Seatbelt path was verified in that PR; this adds the **Linux bubblewrap runtime**
validation that was deferred, plus CI so it runs on every change.

## What runs

`ci/linux-sandbox-check.sh` (as a non-root user, on a prepared offline tree):

1. Unprivileged bubblewrap can create pid/net namespaces.
2. `cargo build --release --workspace --offline --locked`.
3. `cargo test -p ssi-ordering-challenge` — includes
   `linux_local_default_is_bubblewrap_or_fails_closed_never_disabled` and the
   watchdog process-isolation tests.
4. Grader per-worker boundary: `--grader --sandbox-self-check` (PID 1, **no
   default route**, read-only input, hidden host paths, writable output),
   `--sandbox-timeout-self-check`, and the `bubblewrap_timeout_...` e2e test.
5. **#47 local-default POC.** A malicious `order()` writes `$HOME` and opens a
   C2 socket, run over the full dev corpus (300 matrices × 2 determinism runs):
   - default (sandbox on) → **0 C2 connections, no host write**, worker still
     runs and the harness scores it;
   - `SSI_ALLOW_UNSANDBOXED_WORKER=1` → **600 C2 connections + host write +
     warning**, proving the sandbox is exactly what blocks the RCE.
6. `scripts/local-candidate-build.sh` Linux (bubblewrap) build boundary.

## How to run

- **CI:** `.github/workflows/linux-sandbox.yml` on `ubuntu-latest` — bubblewrap
  runs natively; no container. It installs bubblewrap + cargo-deny, relaxes the
  Ubuntu-24.04 AppArmor userns restriction, prepares the offline tree, then runs
  the check.
- **Local (non-Linux dev host):** `bash ci/linux-sandbox-check.docker.sh` runs
  it in a privileged Linux container (`--privileged` for user namespaces,
  `--init` so the watchdog's SIGKILL'd descendants are reaped). Requires a
  prepared tree (`scripts/prepare-build.sh` has run).

## Result

Validated on aarch64 Debian 13 (privileged container): **all checks pass**;
default local run → `c2=0 fs=NO`, opt-out → `c2=600 fs=YES` — symmetric with the
macOS Seatbelt evidence. The grader per-worker bubblewrap path was already
covered by `.github/workflows/benchmark.yml`; this job adds coverage of the
**local default** and the build wrapper.
