# Linux grader sandbox

The production grader uses bubblewrap for two distinct untrusted execution
boundaries. The GitHub Actions runner agent and the trusted harness parent stay
outside both boundaries and run as the ordinary `runner` user, never root.

## Build boundary

After `prepare-build.sh` has validated declarations, vendored sources, and
scanned the tree, the workflow runs two release builds:

```text
scripts/grader-sandbox.sh trusted-build
scripts/grader-sandbox.sh candidate-build
```

The first builds only the frozen `ssi-ordering-challenge` parent and its trusted
workspace dependencies into `target/trusted`. It does not compile or link
`src/ordering` or any dependency declared by the contestant. The second builds
`ssi-candidate-worker`, which includes `src/ordering` and those dependencies,
into `target/candidate`.

The build gets:

- no network and private PID, IPC, UTS, and cgroup namespaces;
- an empty environment with only explicit Cargo/toolchain variables;
- read-only `/workspace`, `/usr`, `/etc` (for the system linker: `cc` via
  `/etc/alternatives` and `ld` via `/etc/ld.so.*`), runtime libraries, and the
  Rust toolchain;
- one clean writable target mount and a private `/tmp`;
- no view of runner HOME, credentials, temp files, or the eval corpus (which is
  downloaded only after the build).

The wrapper refuses root, non-Linux hosts, and missing bubblewrap. The workflow
runs `scripts/grader-sandbox.sh self-check` before the build and fails if the
checkout is writable or an outbound TCP connection succeeds. A tmpfs hides the
host `target` tree and only the selected clean target directory is mounted
writable. Candidate build scripts therefore cannot replace the trusted parent
artifact.

`prepare-build.sh` itself is outside this boundary because it needs network for
`cargo vendor`. It only compiles/runs the trusted `ssi-purity` package; the
generated root manifest is controlled by `Cargo.toml.in`, and candidate
dependency code does not execute until the sandboxed full build.

## Per-worker boundary

The scored harness runs with an explicit production mode and paths to the two
separately built artifacts:

```text
SSI_GRADER_SANDBOX=bubblewrap
SSI_CANDIDATE_WORKER=.../target/candidate/release/ssi-candidate-worker
target/trusted/release/ssi-ordering-challenge --grader
```

`src/sandbox.rs` then constructs a fresh bubblewrap command for **each**
`order()` invocation (including both determinism runs). Each worker gets:

- its own PID namespace and `/proc`, with the worker as PID 1;
- its own network namespace with no interfaces configured and no default route;
- an empty environment and `/dev/null` for stdin, stdout, and stderr;
- an empty tmpfs root containing only read-only runtime libraries, the
  candidate executable, and the one staged pattern;
- exactly one writable bind mount: the pre-created permutation output file;
- no Linux capabilities, repo, corpus file, HOME, runner temp, or parent files;
- per-worker resource limits applied by wrapping bubblewrap in `prlimit`
  (util-linux), inherited across bwrap into the worker:
  - a size-capped root tmpfs (`--tmpfs / --size`, 64 MiB) so writes to `/`
    cannot exhaust RAM, and a read-only `/dev` (`--remount-ro /dev`) so the
    second, RAM-charged tmpfs bubblewrap mounts there cannot be filled either;
  - `RLIMIT_FSIZE` sized to the exact `8 + 8·n` permutation output (plus a
    small slack) so a runaway write to the output file hits `SIGXFSZ` instead
    of filling the disk;
  - `RLIMIT_AS` at the documented 2–4 GB per-matrix memory ceiling (4 GiB);
  - `RLIMIT_NPROC` (4096) to bound a fork bomb;
  - `RLIMIT_CORE` 0 to suppress core dumps.

  `prlimit` is required in `--grader` mode; its absence is a hard failure, like
  a missing bubblewrap.

This must remain per worker. Wrapping the whole harness in one PID namespace is
not equivalent: workers in that shared namespace can enumerate the trusted
parent in `/proc` and may be able to inspect its memory. Keeping the trusted
parent outside and creating a child namespace for every invocation prevents
that visibility while still letting the parent validate and score output. The
candidate executable is the process started inside bubblewrap, so its loader,
Rust startup, dependency `.init_array` entries, and `main` all begin after the
namespace and filesystem boundary exists.

## Local per-worker boundary (default)

The pushed-back winning submission is untrusted code, and the local development
loop in `RULES.md` runs it (`cargo run --release`). Local runs therefore
sandbox `order()` **by default** rather than executing it directly on the
contestant's host (issue #47). With `SSI_GRADER_SANDBOX` unset, `from_env`
selects the host-native sandbox:

- **Linux** → the same per-worker **bubblewrap** command described above (fresh
  PID/network/mount namespace, network unshared, minimal read-only filesystem,
  one writable output).
- **macOS** → **`sandbox-exec` (Seatbelt)**, since bubblewrap does not exist
  there. The profile denies all network and every filesystem write except the
  one pre-created output file, while allowing the reads (runtime libraries, the
  dyld shared cache), `mach-lookup`, and `sysctl-read` a dynamically linked Rust
  binary needs to start. The parent still `env_clear()`s, nulls stdio, and runs
  the worker in a private cwd. The output path is canonicalized before it goes
  into the write-allow rule (Seatbelt matches resolved paths, and macOS
  `/var`→`/private/var`).

If no native sandbox is available (e.g. a Linux host without `bwrap`), the run
**fails closed** with an actionable message — it never silently degrades to
direct execution. The explicit, at-your-own-risk opt-out
`SSI_ALLOW_UNSANDBOXED_WORKER=1` restores direct execution and prints a loud
warning on every run; it is honored only in local mode and ignored under
`--grader`.

The build phase is sandboxed by default too, via
`scripts/local-candidate-build.sh`: on Linux it reuses the bubblewrap build
boundary, on macOS it runs `cargo build -p ssi-candidate-worker` under
`sandbox-exec` with network denied and writes confined to the build/cache/temp
roots. This is the default local build path — `benchmark.json`'s
`benchmarkCommand` runs the wrapper and its `setupCommand` warms only the trusted
parent (`-p ssi-ordering-challenge`, not `--workspace`), so no default path
compiles the untrusted candidate (its `deps.toml` dependencies can carry build
scripts/proc-macros that execute at compile time) outside a sandbox. The trusted
parent build is unaffected.

Production additionally passes `--grader`; in that mode an unset or empty
`SSI_GRADER_SANDBOX` is a hard failure, and the opt-out is ignored. Unsupported
platforms, root execution, missing bubblewrap, missing candidate/input/output
files, and spawn failures are also hard failures.

## Verification

CI executes the trusted release binary with `--grader --sandbox-self-check`
before fetching the hidden corpus. That check starts the candidate binary with
the same command builder as real workers and
requires:

- PID 1 and only one visible numeric `/proc` entry;
- no default network route;
- no environment entries other than `PWD` (which bubblewrap sets to its
  `--chdir` target after `--clearenv`; nothing is inherited from the parent);
- all three standard streams linked to `/dev/null`;
- no `/workspace`, `/home`, `/root`, or `/tmp`;
- read-only pattern input and writable permutation output;
- live per-worker resource limits: `/proc/self/limits` shows finite (not
  `unlimited`) caps on file size, process count, and address space, the
  root filesystem is a size-capped tmpfs, and `/dev` is mounted read-only and
  refuses a new file.

The workflow also runs `--grader --sandbox-timeout-self-check`: the candidate
starts a descendant and blocks, then the watchdog must time out and tear down
the bubblewrap process group/PID namespace. Unit tests pin parent/candidate
linkage, command construction, missing grader sandbox failure, protocol
round-trips, and local timeout behavior. The live bubblewrap checks are
Linux-only.

## Operational assumptions and residual risk

- The Linux host must allow the user namespaces bubblewrap needs. GitHub's
  Ubuntu runner and packaged bubblewrap are the supported grader platform.
- `/usr` is exposed read-only during builds because Rust linking needs the
  system linker. Worker sandboxes are narrower and expose runtime library
  directories only.
- Bubblewrap is namespace/mount isolation, not a syscall filter or virtual
  machine. Kernel vulnerabilities remain in scope for runner hardening.
- The trusted workflow and frozen package manifests must remain outside the
  contestant-editable submission paths. The package split prevents Rust linkage
  and target replacement; it does not defend a workflow or trusted-source
  compromise.
- Per-worker limits are enforced with `prlimit` rlimits, not a cgroup. This is
  portable to the unprivileged runner user (no cgroup delegation needed) but has
  two consequences. `RLIMIT_NPROC` is accounted per user, not per worker, so it
  cannot be as tight as a per-cgroup `pids.max`; 4096 bounds a runaway well below
  the observed ~24k while staying clear of any legitimate task count. (How much
  of the budget the worker actually gets to itself depends on the kernel: since
  5.14 the count is kept per `(user_namespace, uid)` via ucounts, and bubblewrap
  puts each worker in its own user namespace, so the cap is likely tighter in
  practice than a shared per-UID count would be. The choice of 4096 does not
  depend on which behaviour holds.) `RLIMIT_AS` caps virtual address space, not
  RSS; the 64 MiB `--size` cap on the root tmpfs is what bounds tmpfs RAM. A
  per-worker memory cgroup (`memory.max`) would be a stricter belt-and-braces
  defense where cgroup delegation is available.
- `prlimit` is given one value per resource, which sets the soft AND hard limit,
  so the worker cannot raise a cap back after `exec`. The cost is that on a host
  whose own hard limit is already below one of these values, the unprivileged
  parent cannot set it and `prlimit` exits nonzero — every worker fails rather
  than running uncapped. That is the intended direction (fail closed), and
  `--grader --sandbox-self-check` gates it with a visible error before any
  grading starts, because a worker's own stderr goes to `/dev/null`.
- `/dev` is held read-only rather than size-capped. bubblewrap consumes `--size`
  only in its `--tmpfs` handler and rejects `--size` before `--dev`, so the /dev
  tmpfs cannot take a size the way `/` does. Read-only is the tighter bound
  anyway (zero writable bytes), and the remount is non-recursive, so the device
  nodes bind-mounted under /dev stay usable. Note that per-file `RLIMIT_FSIZE`
  alone does not bound a tmpfs: many small files each stay under the per-file
  cap while the total is unlimited, which is why each writable tmpfs needs its
  own bound.
- The remaining RAM-charged shmem path is the one with no mount point to cap:
  anonymous shared memory (`memfd_create`, `MAP_SHARED | MAP_ANONYMOUS`). It is
  bounded without a size cap — `RLIMIT_FSIZE` limits each memfd, `RLIMIT_NOFILE`
  limits how many the worker can hold open at once, and anything it maps counts
  against `RLIMIT_AS` — so no separate control is needed. /dev/shm needs none
  either: bubblewrap creates it inside the /dev tmpfs, which is read-only.
- Build output and compiler diagnostics are visible to the trusted workflow.
  Worker standard streams are not.
