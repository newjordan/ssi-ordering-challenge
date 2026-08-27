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
  Seatbelt profile is `deny default` re-allowing exec/fork, broad `file-read*`
  **with the home directories carved out (issue #63, see below)**,
  `mach-lookup`, `sysctl-read`; **`deny network*`**; and `file-write*` for
  exactly the one canonicalized output file (+ `/dev/null`). No native sandbox →
  **fail closed** with an actionable message. `SSI_ALLOW_UNSANDBOXED_WORKER=1` is
  an explicit, loud opt-out honored only in local mode.
- **Build phase** (`scripts/local-candidate-build.sh`): builds the untrusted
  `ssi-candidate-worker` under the same containment — bubblewrap on Linux (reuses
  `grader-sandbox.sh candidate-build`), `sandbox-exec` with network denied on
  macOS. This is the **default local build path**: `benchmark.json`'s
  `benchmarkCommand` runs the wrapper (not a bare `cargo build -p
  ssi-candidate-worker`), and `setupCommand` warms only the trusted parent
  (`cargo build -p ssi-ordering-challenge`, no `--workspace`) so no default path
  compiles the candidate unsandboxed. A dependency build script or proc-macro
  declared via `src/ordering/deps.toml` runs arbitrary code at build time, so
  this must be contained just like the execute phase. The trusted parent build
  is untouched.
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
- **Build phase (default path):** the `benchmarkCommand` build step
  (`scripts/local-candidate-build.sh`) was driven with a `build.rs` payload that
  opens a TCP connection and writes a host file (the compile-time analogue of a
  malicious dependency). Under the default wrapper it reports
  `network=BLOCKED host_write=BLOCKED` (0 C2, no host write); the same payload
  under a bare `cargo build` connects and writes. Writes stay confined to
  `target/`, `CARGO_HOME`, `TMPDIR`.
- **Regression:** `cargo test --release` green (Seatbelt selection/command tests;
  `security_boundary` asserts the default build path routes through the sandbox).

## Home-directory read carve-out (issue #63)

The original Seatbelt profile paired broad `file-read*` with network-deny and
write-confinement and claimed that left "no exfiltration path." **That was
false.** The one sanctioned output — the returned permutation — is an
attacker-controlled, score-modulating channel that reaches a *public* artifact:
a winning `order()` could read `~/.ssh`, `~/.aws`, the keychain or browser data
(all allowed by broad `file-read*`) and encode what it found as pre-tuned
permutation swaps, shifting the committed `score.json` / `results.tsv` by
distinct, decodable amounts. Since every contestant runs the pushed-back winner
locally, that is fleet-scale, invisible exfiltration through the score itself —
network-deny does nothing about it. (The Linux/grader bubblewrap path was never
affected: it binds in only the runtime-library trees, the binary, the input and
the output, so there is nothing under `$HOME` to read.)

**Fix:** `seatbelt_profile()` keeps broad `file-read*` but denies reads of the
home directories (`/Users`, `/home`, `/var/root`, `/private/var/root`) and
re-allows the exact candidate binary and input by literal *after* the denies
(Seatbelt takes the last matching rule) so the worker still loads and reads its
pattern when the clone lives under `$HOME`. Every credential class the POC
exfiltrated lives under a home directory, so the channel is left with nothing
worth encoding.

Because Seatbelt resolves symlinks before matching a `subpath`, the static
prefixes alone would miss a home relocated onto another volume (e.g.
`/Users/bob` → `/Volumes/data/bob`, common for network or oversized homes):
`~/.ssh` would canonicalize outside `/Users` and slip past the deny. So the
profile also resolves `$HOME` to its realpath at build time and denies that when
it falls outside the static prefixes (`seatbelt_denied_read_subtrees`). A home
whose individual subdirectory is symlinked off-tree still cannot be enumerated
this way — only the home root is resolved.

Why deny-list, not the Linux side's tight allow-list: empirically (macOS 26.x /
arm64), a deny-default profile that allows only `/usr/lib`, `/System` and
`/private/var/db/dyld` — or even every standard system tree while omitting
`/Users` — aborts in `dyld4::CacheFinder` before `order()` runs, because dyld's
shared-cache discovery reads across version-dependent paths (the cryptex-split
cache, closures, fallbacks) a minimal allow-list cannot pin. Keeping broad read
while carving out the home trees runs cleanly. The positive direction (worker
runs) is pinned by `seatbelt_worker_produces_valid_permutation`; the negative
(a `$HOME` read is blocked, a system read is not) by
`seatbelt_denies_home_reads_but_runs`.

## Residual (deferred to Yukon)

- Non-home host files (e.g. `/etc`, `/Library`, `/private/var`) remain readable,
  since dyld needs broad read on macOS. The high-value, per-user secrets are all
  under `$HOME` and are now denied; fully confining the read side to the Linux
  allow-list model is part of the later Yukon managed sandbox.
- Seatbelt is namespace/policy isolation, not a VM; kernel bugs remain in scope.
- `sandbox-exec` is deprecated by Apple but present and functional on current
  macOS; Yukon replaces this local shim with its managed sandbox.
