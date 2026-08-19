# Linux grader sandbox

The production grader uses bubblewrap for two distinct untrusted execution
boundaries. The GitHub Actions runner agent and the trusted harness parent stay
outside both boundaries and run as the ordinary `runner` user, never root.

## Build boundary

After `prepare-build.sh` has validated declarations, vendored sources, and
scanned the tree, `scripts/grader-sandbox.sh build` runs the release build.
This is the first point at which dependency `build.rs` programs and proc macros
can execute.

The build gets:

- no network and private PID, IPC, UTS, and cgroup namespaces;
- an empty environment with only explicit Cargo/toolchain variables;
- read-only `/workspace`, `/usr`, runtime libraries, and Rust toolchain;
- writable `/workspace/target` and a private `/tmp`;
- no view of runner HOME, credentials, temp files, or the eval corpus (which is
  downloaded only after the build).

The wrapper refuses root, non-Linux hosts, and missing bubblewrap. The workflow
runs `scripts/grader-sandbox.sh self-check` before the build and fails if the
checkout is writable or an outbound TCP connection succeeds.

`prepare-build.sh` itself is outside this boundary because it needs network for
`cargo vendor`. It only compiles/runs the trusted `ssi-purity` package; the
generated root manifest is controlled by `Cargo.toml.in`, and candidate
dependency code does not execute until the sandboxed full build.

## Per-worker boundary

The scored harness runs with:

```text
SSI_GRADER_SANDBOX=bubblewrap
```

`src/sandbox.rs` then constructs a fresh bubblewrap command for **each**
`order()` invocation (including both determinism runs). Each worker gets:

- its own PID namespace and `/proc`, with the worker as PID 1;
- its own network namespace with no interfaces configured and no default route;
- an empty environment and `/dev/null` for stdin, stdout, and stderr;
- an empty tmpfs root containing only read-only runtime libraries, the harness
  executable, and the one staged pattern;
- exactly one writable bind mount: the pre-created permutation output file;
- no Linux capabilities, repo, corpus file, HOME, runner temp, or parent files.

This must remain per worker. Wrapping the whole harness in one PID namespace is
not equivalent: workers in that shared namespace can enumerate the trusted
parent in `/proc` and may be able to inspect its memory. Keeping the trusted
parent outside and creating a child namespace for every invocation prevents
that visibility while still letting the parent validate and score output.

When the environment variable is absent, local Linux/macOS development keeps
the existing direct subprocess behavior. When it is present, unsupported
platforms, root execution, missing bubblewrap, missing bind inputs, and spawn
failures are hard failures.

## Verification

CI executes the release binary with `--sandbox-self-check` before fetching the
hidden corpus. That check uses the same command builder as real workers and
requires:

- PID 1 and only one visible numeric `/proc` entry;
- no default network route;
- no environment entries;
- all three standard streams linked to `/dev/null`;
- no `/workspace`, `/home`, `/root`, or `/tmp`;
- read-only pattern input and writable permutation output.

Unit tests pin local command construction and effective-UID parsing. Normal
workspace tests continue to run without requesting Linux sandbox mode.

## Operational assumptions and residual risk

- The Linux host must allow the user namespaces bubblewrap needs. GitHub's
  Ubuntu runner and packaged bubblewrap are the supported grader platform.
- `/usr` is exposed read-only during builds because Rust linking needs the
  system linker. Worker sandboxes are narrower and expose runtime library
  directories only.
- Bubblewrap is namespace/mount isolation, not a syscall filter or virtual
  machine. Kernel vulnerabilities remain in scope for runner hardening.
- The watchdog limits each worker to two seconds, but this slice does not add a
  cgroup memory limit. Runner-level resource limits remain a separate defense.
- Build output and compiler diagnostics are visible to the trusted workflow.
  Worker standard streams are not.
