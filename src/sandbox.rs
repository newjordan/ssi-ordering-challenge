//! Per-worker Linux sandboxing for the production grader.
//!
//! The trusted harness stays outside the sandbox so it can load and score the
//! hidden corpus. In grader mode, every individual `order()` invocation gets a
//! fresh bubblewrap PID and network namespace plus a minimal filesystem. This
//! is intentionally per worker: putting the parent and all workers in one PID
//! namespace would still expose the parent's `/proc` entries to contestant
//! code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const MODE_ENV: &str = "SSI_GRADER_SANDBOX";

/// Size cap for the worker's root tmpfs. bubblewrap mounts `/` as tmpfs, whose
/// pages are charged to RAM (page cache), not runner disk. Without a `--size`
/// bound a submission can memory-exhaust the whole grading job — trusted parent
/// included — by writing to `/` (issue #49). The worker legitimately writes
/// nothing to `/`; its only writable path is the bound output file. 64 MiB
/// leaves slack for transient runtime files while making a fill attack fail
/// against this cap instead of the machine.
const ROOT_TMPFS_BYTES: u64 = 64 * 1024 * 1024;

/// Address-space cap for the worker (`RLIMIT_AS`). The cost doc fixes a 2–4 GB
/// per-matrix memory cap that "doubles as the anti-lookup-table cap"; the
/// factorization that needs it runs in the trusted parent, so an ordering-only
/// worker never approaches this. Enforcing the documented ceiling here also
/// stops an in-process allocation OOM that the tmpfs `--size` cap does not
/// cover.
const ADDRESS_SPACE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Process cap for the worker (`RLIMIT_NPROC`). A legitimate ordering forks
/// nothing (rayon uses threads within one process); a fork bomb (issue #52) can
/// otherwise spawn tens of thousands. `RLIMIT_NPROC` is per-real-UID — it counts
/// the trusted parent's own tasks too — so it cannot be set as tight as a
/// per-cgroup `pids.max` (which would need cgroup delegation the unprivileged
/// runner user does not get). 4096 bounds a runaway well below the observed
/// ~24k while staying clear of any legitimate steady-state task count.
const MAX_PROCESSES: u64 = 4096;

/// Headroom added above the exact permutation size when capping the writable
/// output file, so the exact valid write always succeeds.
const OUTPUT_SLACK_BYTES: u64 = 4096;

/// Output-file cap for the sandbox self-checks, which write a short diagnostic
/// string rather than a permutation.
const SELF_CHECK_OUTPUT_BYTES: u64 = 64 * 1024;

/// Per-worker resource limits applied in grader mode. `for_order` sizes the
/// writable output to the exact permutation this matrix produces so a runaway
/// write hits `SIGXFSZ` immediately (issue #48); the other caps are fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerLimits {
    root_tmpfs_bytes: u64,
    max_output_bytes: u64,
    address_space_bytes: u64,
    max_processes: u64,
}

impl WorkerLimits {
    /// Limits for a real `order()` invocation on a pattern of dimension `n`. A
    /// valid permutation output is exactly `8 + 8*n` bytes (see
    /// `ssi_worker_protocol::write_permutation`).
    pub fn for_order(n: usize) -> Self {
        let max_output_bytes = 8u64
            .saturating_add((n as u64).saturating_mul(8))
            .saturating_add(OUTPUT_SLACK_BYTES);
        Self {
            root_tmpfs_bytes: ROOT_TMPFS_BYTES,
            max_output_bytes,
            address_space_bytes: ADDRESS_SPACE_BYTES,
            max_processes: MAX_PROCESSES,
        }
    }

    /// Limits for the sandbox self-checks (diagnostic output, not a permutation).
    pub fn for_self_check() -> Self {
        Self {
            root_tmpfs_bytes: ROOT_TMPFS_BYTES,
            max_output_bytes: SELF_CHECK_OUTPUT_BYTES,
            address_space_bytes: ADDRESS_SPACE_BYTES,
            max_processes: MAX_PROCESSES,
        }
    }

    #[cfg(test)]
    fn max_output_bytes(&self) -> u64 {
        self.max_output_bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerInvocation {
    Order,
    SandboxSelfCheck,
    SandboxTimeoutSelfCheck,
    /// Harness-owned time-cap regression mode: the candidate binary sleeps past
    /// the cap without ever calling `order()`, so the watchdog is exercised
    /// against a real child while contestant code never sees a test seam.
    TestTimeout,
}

impl WorkerInvocation {
    fn flag(self) -> Option<&'static str> {
        match self {
            Self::Order => None,
            Self::SandboxSelfCheck => Some("--sandbox-self-check"),
            Self::SandboxTimeoutSelfCheck => Some("--sandbox-timeout-self-check"),
            Self::TestTimeout => Some("--worker-test-timeout"),
        }
    }
}

#[derive(Debug)]
pub enum WorkerSandbox {
    Disabled,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Bubblewrap {
        executable: PathBuf,
        /// `prlimit` (util-linux) wraps bubblewrap to apply per-worker rlimits
        /// (file size, address space, process count). Kept as a resolved path
        /// so the worker command never depends on `PATH`.
        prlimit: PathBuf,
    },
}

impl WorkerSandbox {
    /// Select the sandbox from the trusted parent's environment.
    ///
    /// Local development remains direct when `SSI_GRADER_SANDBOX` is unset.
    /// `grader_mode` makes that same missing selection a hard error, so the
    /// production command cannot silently degrade to an unsandboxed worker.
    pub fn from_env(grader_mode: bool) -> Result<Self, String> {
        Self::from_value(grader_mode, std::env::var_os(MODE_ENV))
    }

    fn from_value(grader_mode: bool, value: Option<std::ffi::OsString>) -> Result<Self, String> {
        let Some(value) = value else {
            return if grader_mode {
                Err(format!(
                    "{MODE_ENV} must be exactly \"bubblewrap\" in --grader mode"
                ))
            } else {
                Ok(Self::Disabled)
            };
        };
        if value.is_empty() {
            return if grader_mode {
                Err(format!(
                    "{MODE_ENV} must be exactly \"bubblewrap\" in --grader mode"
                ))
            } else {
                Ok(Self::Disabled)
            };
        }
        if value != "bubblewrap" {
            return Err(format!(
                "{MODE_ENV} must be unset or exactly \"bubblewrap\""
            ));
        }

        #[cfg(not(target_os = "linux"))]
        {
            return Err(format!(
                "{MODE_ENV}=bubblewrap is supported only on Linux; leave it unset for local development"
            ));
        }

        #[cfg(target_os = "linux")]
        {
            if effective_uid()? == 0 {
                return Err(
                    "grader parent is running as root; refusing to launch untrusted code"
                        .to_string(),
                );
            }
            let executable = ["/usr/bin/bwrap", "/bin/bwrap"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.is_file())
                .ok_or_else(|| {
                    "bubblewrap requested but /usr/bin/bwrap and /bin/bwrap are absent".to_string()
                })?;
            let prlimit = ["/usr/bin/prlimit", "/bin/prlimit"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.is_file())
                .ok_or_else(|| {
                    "per-worker resource limits require prlimit (util-linux) but \
                     /usr/bin/prlimit and /bin/prlimit are absent"
                        .to_string()
                })?;
            Ok(Self::Bubblewrap {
                executable,
                prlimit,
            })
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Bubblewrap { .. })
    }

    /// Construct one worker command. The caller must create `output` first so
    /// bubblewrap can bind that single writable file into an otherwise
    /// read-only/minimal filesystem.
    pub fn worker_command(
        &self,
        candidate_exe: &Path,
        invocation: WorkerInvocation,
        input: &Path,
        output: &Path,
        local_cwd: &Path,
        limits: &WorkerLimits,
    ) -> Result<Command, String> {
        match self {
            Self::Disabled => {
                // Local development runs contestant code directly; it is
                // arbitrary-code-by-design (issue #47) and gets no sandbox, so
                // the grader resource limits do not apply here.
                let _ = limits;
                let mut cmd = Command::new(candidate_exe);
                if let Some(flag) = invocation.flag() {
                    cmd.arg(flag);
                }
                // The worker runs contestant code; hand it NOTHING beyond its
                // pattern/output arguments. No environment entry — test control
                // included — ever reaches order() (F2).
                cmd.arg(input)
                    .arg(output)
                    .env_clear()
                    .current_dir(local_cwd);
                null_stdio(&mut cmd);
                Ok(cmd)
            }
            Self::Bubblewrap {
                executable,
                prlimit,
            } => {
                let candidate_exe = canonical_file(candidate_exe, "candidate executable")?;
                let input = canonical_file(input, "worker input")?;
                let output = canonical_file(output, "worker output")?;

                // Wrap bubblewrap in prlimit so per-worker rlimits are set on
                // the process that execs bwrap and inherited across it into the
                // worker. bubblewrap provides namespace/mount isolation but no
                // resource limits; these caps close the "missing per-worker
                // resource limits" family (issues #48, #52, and the in-process
                // memory vector of #49) that isolation alone does not cover:
                //   --fsize caps the single writable output file (issue #48),
                //   --nproc bounds a fork bomb (issue #52),
                //   --as    enforces the documented per-matrix memory cap,
                //   --core  suppresses core dumps that would fill the scratch.
                let mut cmd = Command::new(prlimit);
                cmd.env_clear()
                    .arg(format!("--nproc={}", limits.max_processes))
                    .arg(format!("--fsize={}", limits.max_output_bytes))
                    .arg(format!("--as={}", limits.address_space_bytes))
                    .arg("--core=0")
                    .arg("--")
                    .arg(executable);

                cmd.args([
                    "--unshare-pid",
                    "--unshare-net",
                    "--unshare-ipc",
                    "--unshare-uts",
                    "--new-session",
                    "--die-with-parent",
                    "--as-pid-1",
                    "--cap-drop",
                    "ALL",
                    "--clearenv",
                ]);
                // Size-cap the root tmpfs so a submission cannot exhaust RAM by
                // writing to `/` (issue #49). `--size` applies to the tmpfs
                // mounted immediately after it.
                cmd.arg("--size")
                    .arg(limits.root_tmpfs_bytes.to_string())
                    .args(["--tmpfs", "/", "--proc", "/proc", "--dev", "/dev"]);
                // `--dev` mounts a SECOND tmpfs, which is also RAM-charged. It
                // cannot be size-capped the same way: bubblewrap consumes
                // `--size` only in its `--tmpfs` handler and dies with "--size
                // must be followed by --tmpfs" otherwise (bubblewrap.c,
                // `next_size_arg`). Left alone it is an unbounded RAM sink that
                // the per-file RLIMIT_FSIZE does not cover, because many small
                // files each stay under the per-file cap while the total does
                // not. Remount it read-only instead — zero writable bytes is a
                // strictly tighter bound than any size cap. The remount is
                // non-recursive and applies to the /dev tmpfs itself, so the
                // device nodes bubblewrap bind-mounted under it (/dev/null,
                // /dev/urandom, ...) stay readable and writable as devices.
                cmd.args(["--remount-ro", "/dev"]);

                // The Rust binary is normally dynamically linked to libc and
                // libgcc. Expose only runtime-library trees, read-only; do not
                // expose /usr/bin, the repository, HOME, or runner temp files.
                for runtime_dir in ["/lib", "/lib64", "/usr/lib"] {
                    if Path::new(runtime_dir).exists() {
                        cmd.arg("--ro-bind").arg(runtime_dir).arg(runtime_dir);
                    }
                }

                cmd.arg("--ro-bind")
                    .arg(&candidate_exe)
                    .arg("/candidate-worker")
                    .arg("--ro-bind")
                    .arg(&input)
                    .arg("/input/pattern.bin")
                    .arg("--bind")
                    .arg(&output)
                    .arg("/output/permutation.bin")
                    .args(["--chdir", "/", "--", "/candidate-worker"]);
                if let Some(flag) = invocation.flag() {
                    cmd.arg(flag);
                }
                cmd.args(["/input/pattern.bin", "/output/permutation.bin"]);
                null_stdio(&mut cmd);
                Ok(cmd)
            }
        }
    }
}

fn canonical_file(path: &Path, description: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|e| format!("cannot resolve {description} {}: {e}", path.display()))?;
    if !canonical.is_file() {
        return Err(format!(
            "{description} is not a regular file: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn null_stdio(cmd: &mut Command) {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
}

#[cfg(target_os = "linux")]
fn effective_uid() -> Result<u32, String> {
    let status = fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("cannot determine grader uid from /proc/self/status: {e}"))?;
    parse_effective_uid(&status)
        .ok_or_else(|| "cannot parse effective uid from /proc/self/status".to_string())
}

#[cfg(target_os = "linux")]
fn parse_effective_uid(status: &str) -> Option<u32> {
    let uid_line = status.lines().find(|line| line.starts_with("Uid:"))?;
    uid_line.split_whitespace().nth(2)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn disabled_command_clears_environment_and_uses_host_paths() {
        let sandbox = WorkerSandbox::Disabled;
        let cmd = sandbox
            .worker_command(
                Path::new("/candidate"),
                WorkerInvocation::Order,
                Path::new("/pattern"),
                Path::new("/permutation"),
                Path::new("/empty"),
                &WorkerLimits::for_order(4),
            )
            .unwrap();

        assert_eq!(cmd.get_program(), OsStr::new("/candidate"));
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            [OsStr::new("/pattern"), OsStr::new("/permutation")]
        );
        assert!(cmd.get_envs().all(|(_, value)| value.is_none()));
        assert_eq!(cmd.get_current_dir(), Some(Path::new("/empty")));
    }

    #[test]
    fn grader_mode_rejects_missing_or_empty_sandbox_selection() {
        assert!(WorkerSandbox::from_value(true, None)
            .unwrap_err()
            .contains("must be exactly \"bubblewrap\""));
        assert!(WorkerSandbox::from_value(true, Some("".into()))
            .unwrap_err()
            .contains("must be exactly \"bubblewrap\""));
        assert!(matches!(
            WorkerSandbox::from_value(false, None).unwrap(),
            WorkerSandbox::Disabled
        ));
    }

    #[test]
    fn bubblewrap_command_executes_candidate_binary_from_process_start() {
        let dir = std::env::temp_dir().join(format!("ssi-sandbox-command-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let candidate = dir.join("candidate");
        let input = dir.join("input");
        let output = dir.join("output");
        for path in [&candidate, &input, &output] {
            std::fs::write(path, b"x").unwrap();
        }
        let sandbox = WorkerSandbox::Bubblewrap {
            executable: PathBuf::from("/test/bwrap"),
            prlimit: PathBuf::from("/test/prlimit"),
        };
        let cmd = sandbox
            .worker_command(
                &candidate,
                WorkerInvocation::Order,
                &input,
                &output,
                &dir,
                &WorkerLimits::for_order(4),
            )
            .unwrap();
        let args: Vec<_> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let candidate = std::fs::canonicalize(candidate)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // The worker is now launched through prlimit, which execs bwrap after
        // its option terminator; bwrap in turn execs the candidate.
        assert_eq!(cmd.get_program(), OsStr::new("/test/prlimit"));
        assert!(args
            .windows(2)
            .any(|window| window == ["--", "/test/bwrap"]));
        assert!(args
            .windows(3)
            .any(|window| window == ["--ro-bind", &candidate, "/candidate-worker"]));
        assert!(args
            .windows(2)
            .any(|window| window == ["--", "/candidate-worker"]));
        assert!(!args.iter().any(|arg| arg == "--worker"));
    }

    fn bubblewrap_order_command(n: usize) -> Vec<String> {
        let dir = std::env::temp_dir().join(format!("ssi-sandbox-limits-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let candidate = dir.join("candidate");
        let input = dir.join("input");
        let output = dir.join("output");
        for path in [&candidate, &input, &output] {
            std::fs::write(path, b"x").unwrap();
        }
        let sandbox = WorkerSandbox::Bubblewrap {
            executable: PathBuf::from("/test/bwrap"),
            prlimit: PathBuf::from("/test/prlimit"),
        };
        let cmd = sandbox
            .worker_command(
                &candidate,
                WorkerInvocation::Order,
                &input,
                &output,
                &dir,
                &WorkerLimits::for_order(n),
            )
            .unwrap();
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn worker_limits_for_order_caps_output_to_permutation_size() {
        // A valid permutation of 0..n serializes to exactly 8 + 8*n bytes.
        assert_eq!(WorkerLimits::for_order(4).max_output_bytes(), 8 + 32 + 4096);
        // Huge n must not overflow the cap arithmetic.
        assert!(WorkerLimits::for_order(usize::MAX).max_output_bytes() >= 4096);
    }

    #[test]
    fn bubblewrap_command_caps_root_tmpfs_size() {
        let argv = bubblewrap_order_command(4);
        assert!(
            argv.windows(4)
                .any(|w| w == ["--size", "67108864", "--tmpfs", "/"]),
            "root tmpfs must be size-capped before it is mounted (issue #49): {argv:?}"
        );
    }

    #[test]
    fn bubblewrap_command_makes_dev_read_only() {
        let argv = bubblewrap_order_command(4);
        // `--dev /dev` mounts a SECOND tmpfs, and bubblewrap's `--size` composes
        // only with `--tmpfs` (`--size N --dev /dev` dies with "--size must be
        // followed by --tmpfs"), so that tmpfs cannot be size-capped the way `/`
        // is. Remount it read-only instead: zero writable bytes is a strictly
        // tighter bound than any size cap, and it closes the many-small-files
        // RAM fill that the per-file RLIMIT_FSIZE does not cover (issue #49).
        let dev = argv
            .iter()
            .position(|arg| arg == "--dev")
            .unwrap_or_else(|| panic!("no --dev mount to harden: {argv:?}"));
        let remount = argv
            .windows(2)
            .position(|w| w == ["--remount-ro", "/dev"])
            .unwrap_or_else(|| {
                panic!("/dev tmpfs must be remounted read-only (issue #49): {argv:?}")
            });
        assert!(
            remount > dev,
            "--remount-ro /dev must follow the --dev mount it hardens: {argv:?}"
        );
    }

    #[test]
    fn bubblewrap_command_wraps_bwrap_in_prlimit_with_resource_caps() {
        let argv = bubblewrap_order_command(4);
        assert_eq!(
            argv[0], "/test/prlimit",
            "worker must be wrapped in prlimit"
        );
        assert!(
            argv.contains(&"--nproc=4096".to_string()),
            "fork-bomb cap missing (issue #52): {argv:?}"
        );
        assert!(
            argv.contains(&"--fsize=4136".to_string()),
            "output-file size cap missing or not sized to 8+8n+slack (issue #48): {argv:?}"
        );
        assert!(
            argv.contains(&"--as=4294967296".to_string()),
            "address-space cap missing: {argv:?}"
        );
        // prlimit's option terminator must precede the bwrap program.
        let term = argv.iter().position(|a| a == "--").expect("prlimit --");
        assert_eq!(
            argv[term + 1],
            "/test/bwrap",
            "bwrap must follow prlimit's `--`: {argv:?}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_effective_uid_not_real_uid() {
        let status = "Name:\ttest\nUid:\t1000\t1001\t1002\t1003\n";
        assert_eq!(parse_effective_uid(status), Some(1001));
        assert_eq!(parse_effective_uid("Name:\ttest\n"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires bubblewrap user namespaces and a built candidate worker"]
    fn bubblewrap_timeout_tears_down_candidate_descendants() {
        let candidate = std::env::var_os("SSI_CANDIDATE_WORKER")
            .map(PathBuf::from)
            .expect("SSI_CANDIDATE_WORKER must name the built candidate");
        let sandbox = WorkerSandbox::from_value(true, Some("bubblewrap".into())).unwrap();
        let dir =
            std::env::temp_dir().join(format!("ssi-bwrap-timeout-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input");
        let output = dir.join("output");
        let cwd = dir.join("cwd");
        std::fs::write(&input, b"sentinel").unwrap();
        std::fs::write(&output, b"").unwrap();
        std::fs::create_dir(&cwd).unwrap();

        let mut cmd = sandbox
            .worker_command(
                &candidate,
                WorkerInvocation::SandboxTimeoutSelfCheck,
                &input,
                &output,
                &cwd,
                &WorkerLimits::for_self_check(),
            )
            .unwrap();
        let outcome = crate::watchdog::run_capped(
            &mut cmd,
            &crate::watchdog::CapConfig {
                time_cap: std::time::Duration::from_millis(200),
                poll: std::time::Duration::from_millis(5),
            },
        );
        assert_eq!(outcome, crate::watchdog::WorkerOutcome::Timeout);
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "candidate started\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
