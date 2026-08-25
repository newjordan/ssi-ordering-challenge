# Fix #47 — sandbox the local run loop by default

## The vulnerability

The competition backend pushes each *winning* submission's `src/ordering/` back
to the repo as the new best, and `RULES.md` tells every contestant/agent to run
the local loop (`cargo build -p ssi-candidate-worker && cargo run --release`).
That executes the pushed-back winner's `order()`. In local mode the worker
sandbox was `WorkerSandbox::Disabled` (`src/sandbox.rs`): the child ran with a
cleared environment and a private cwd but **full network and filesystem access
as the victim user**. A malicious winner could phone home / exfiltrate on any
contestant's machine — the "600-C2-receipt" RCE (300 dev matrices × 2
determinism runs = 600 outbound attempts per benchmark).

The grader path was already contained (per-worker bubblewrap for execute,
`grader-sandbox.sh` bubblewrap for build). Only the **local** loop was exposed.

## The fix

Local runs now sandbox the worker **by default**, with no silent fallback to
direct host execution (the Yukon principle, Layr-Labs/yukon#565). This is the
launch-unblocking fix in this repo; the broader cross-checkout containment lands
later via Yukon.

- **Execute phase** (`src/sandbox.rs`): `from_env`/`select` pick the host-native
  sandbox when `SSI_GRADER_SANDBOX` is unset — **bubblewrap** on Linux,
  **`sandbox-exec` (Seatbelt)** on macOS (new `WorkerSandbox::Seatbelt`). The
  Seatbelt profile is `deny default` re-allowing exec/fork, broad `file-read*`,
  `mach-lookup`, `sysctl-read`; **`deny network*`**; and `file-write*` for
  exactly the one canonicalized output file (+ `/dev/null`). No native sandbox →
  **fail closed** with an actionable message. `SSI_ALLOW_UNSANDBOXED_WORKER=1` is
  an explicit, loud opt-out honored only in local mode.
- **Build phase** (`scripts/local-candidate-build.sh`): builds the untrusted
  `ssi-candidate-worker` under the same containment — bubblewrap on Linux (reuses
  `grader-sandbox.sh candidate-build`), `sandbox-exec` with network denied on
  macOS. The trusted parent build is untouched.
- **Grader mode is unchanged**: `--grader` still requires
  `SSI_GRADER_SANDBOX=bubblewrap`, and ignores the opt-out.

## Verification (macOS host)

- **Positive:** `cargo run --release` completes end-to-end under the default
  Seatbelt sandbox; workers produce valid permutations (unit e2e test
  `seatbelt_worker_produces_valid_permutation`, plus a full corpus run).
- **Negative (POC):** an `order()` that writes to `$HOME` and opens a TCP
  connection to a local receiver:
  - default sandbox → **0 C2 connections, no `$HOME` write**; run still yields a
    valid permutation.
  - `SSI_ALLOW_UNSANDBOXED_WORKER=1` → loud warning, `$HOME` write succeeds, and
    **exactly 600 C2 connections** — proving the sandbox is what blocks the RCE.
- **Build phase:** a full clean rebuild under `sandbox-exec` succeeds with
  network denied and writes confined to `target/`, `CARGO_HOME`, `TMPDIR`.
- **Regression:** `cargo test --release` green (new Seatbelt selection/command
  tests; `security_boundary` unchanged for the grader path).

## Residual (deferred to Yukon)

- The local Seatbelt profile allows broad `file-read*` (needed for dyld), so a
  worker can still *read* host files — but with network denied and no writable
  channel except a bijection-validated output, there is no exfiltration path.
  Cross-checkout read restriction is part of the later Yukon containment.
- Seatbelt is namespace/policy isolation, not a VM; kernel bugs remain in scope.
- `sandbox-exec` is deprecated by Apple but present and functional on current
  macOS; Yukon replaces this local shim with its managed sandbox.
