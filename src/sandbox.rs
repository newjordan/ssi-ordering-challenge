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

/// Local-only escape hatch. Running the pushed-back winning submission's
/// `order()` unsandboxed is the #47 RCE, so local runs now sandbox the worker by
/// default and FAIL CLOSED when no native sandbox is available. Setting this to
/// `1` opts back into direct host execution (with a loud warning) for users who
/// understand the risk or cannot install a sandbox. It is honored ONLY in local
/// mode; `--grader` ignores it entirely.
pub const ALLOW_UNSANDBOXED_ENV: &str = "SSI_ALLOW_UNSANDBOXED_WORKER";

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
    /// Direct host execution — no isolation. Reachable locally ONLY via the
    /// explicit `SSI_ALLOW_UNSANDBOXED_WORKER=1` opt-out; never the default.
    Disabled,
    /// Linux bubblewrap: fresh PID/network/mount namespaces per worker. Used by
    /// the grader and, by default, by local runs on Linux.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Bubblewrap { executable: PathBuf },
    /// macOS `sandbox-exec` (Seatbelt): the default for local runs on macOS,
    /// where bubblewrap does not exist. Denies network and all filesystem writes
    /// except the single staged output file.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Seatbelt { executable: PathBuf },
}

impl WorkerSandbox {
    /// Select the sandbox from the trusted parent's environment.
    ///
    /// In `--grader` mode, `SSI_GRADER_SANDBOX` must be exactly `"bubblewrap"`;
    /// anything else is a hard error, so the production command cannot silently
    /// degrade to an unsandboxed worker.
    ///
    /// Local runs sandbox the worker BY DEFAULT (issue #47): with no explicit
    /// `SSI_GRADER_SANDBOX`, the host-native sandbox is selected — bubblewrap on
    /// Linux, `sandbox-exec` (Seatbelt) on macOS. If no native sandbox is
    /// available the selection FAILS CLOSED rather than running untrusted code
    /// directly, unless `SSI_ALLOW_UNSANDBOXED_WORKER=1` is set as an explicit,
    /// at-your-own-risk opt-out.
    pub fn from_env(grader_mode: bool) -> Result<Self, String> {
        Self::from_value(grader_mode, std::env::var_os(MODE_ENV))
    }

    fn from_value(grader_mode: bool, value: Option<std::ffi::OsString>) -> Result<Self, String> {
        // The opt-out is honored only in local mode; --grader ignores it.
        let allow_unsandboxed = !grader_mode && env_is_one(ALLOW_UNSANDBOXED_ENV);
        Self::select(grader_mode, value, allow_unsandboxed)
    }

    /// Pure selection core (no environment reads beyond the injected values), so
    /// tests are deterministic across platforms.
    fn select(
        grader_mode: bool,
        value: Option<std::ffi::OsString>,
        allow_unsandboxed: bool,
    ) -> Result<Self, String> {
        // An explicit, non-empty selection is validated the same way in both
        // modes: only "bubblewrap" is accepted, and it is Linux-only.
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            if value != "bubblewrap" {
                return Err(format!(
                    "{MODE_ENV} must be unset or exactly \"bubblewrap\""
                ));
            }
            return Self::bubblewrap_explicit();
        }

        // No explicit selection.
        if grader_mode {
            return Err(format!(
                "{MODE_ENV} must be exactly \"bubblewrap\" in --grader mode"
            ));
        }
        if allow_unsandboxed {
            return Ok(Self::Disabled);
        }
        Self::native_local()
    }

    /// `SSI_GRADER_SANDBOX=bubblewrap` was requested explicitly.
    fn bubblewrap_explicit() -> Result<Self, String> {
        #[cfg(not(target_os = "linux"))]
        {
            Err(format!(
                "{MODE_ENV}=bubblewrap is supported only on Linux; leave it unset for local development"
            ))
        }
        #[cfg(target_os = "linux")]
        {
            if effective_uid()? == 0 {
                return Err(
                    "grader parent is running as root; refusing to launch untrusted code"
                        .to_string(),
                );
            }
            find_tool(&["/usr/bin/bwrap", "/bin/bwrap"])
                .map(|executable| Self::Bubblewrap { executable })
                .ok_or_else(|| {
                    "bubblewrap requested but /usr/bin/bwrap and /bin/bwrap are absent".to_string()
                })
        }
    }

    /// Default local selection: pick the host-native sandbox, fail closed if
    /// none is available.
    fn native_local() -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            if effective_uid()? == 0 {
                return Err(
                    "refusing to run the untrusted candidate worker as root; run local benchmarks \
                     as a normal user"
                        .to_string(),
                );
            }
            return match find_tool(&["/usr/bin/bwrap", "/bin/bwrap"]) {
                Some(executable) => Ok(Self::Bubblewrap { executable }),
                None => Err(fail_closed_message(
                    "bubblewrap (bwrap)",
                    "install it, e.g. `apt-get install bubblewrap`",
                )),
            };
        }
        #[cfg(target_os = "macos")]
        {
            return match find_tool(&["/usr/bin/sandbox-exec"]) {
                Some(executable) => Ok(Self::Seatbelt { executable }),
                None => Err(fail_closed_message(
                    "sandbox-exec",
                    "it ships with macOS; /usr/bin/sandbox-exec is expected to exist",
                )),
            };
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(fail_closed_message(
                "a built-in worker sandbox",
                "this platform has none",
            ))
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Bubblewrap { .. } | Self::Seatbelt { .. })
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
    ) -> Result<Command, String> {
        match self {
            Self::Disabled => {
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
            Self::Bubblewrap { executable } => {
                let candidate_exe = canonical_file(candidate_exe, "candidate executable")?;
                let input = canonical_file(input, "worker input")?;
                let output = canonical_file(output, "worker output")?;

                let mut cmd = Command::new(executable);
                cmd.env_clear().args([
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
                    "--tmpfs",
                    "/",
                    "--proc",
                    "/proc",
                    "--dev",
                    "/dev",
                ]);

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
            Self::Seatbelt { executable } => {
                // Resolve symlinks: Seatbelt matches on the canonical path, and
                // macOS temp/output paths route through /var -> /private/var, so
                // an unresolved literal in the write-allow rule would never match.
                let candidate_exe = canonical_file(candidate_exe, "candidate executable")?;
                let input = canonical_file(input, "worker input")?;
                let output = canonical_file(output, "worker output")?;
                let profile = seatbelt_profile(&output);

                let mut cmd = Command::new(executable);
                cmd.arg("-p").arg(&profile).arg(&candidate_exe);
                if let Some(flag) = invocation.flag() {
                    cmd.arg(flag);
                }
                cmd.arg(&input)
                    .arg(&output)
                    // Hand contestant code NOTHING beyond its two path arguments,
                    // matching the Disabled/Bubblewrap arms.
                    .env_clear()
                    .current_dir(local_cwd);
                null_stdio(&mut cmd);
                Ok(cmd)
            }
        }
    }
}

/// Build the Seatbelt profile for one worker: deny everything, then re-allow
/// only what a dynamically linked Rust binary needs to run (exec/fork, reading
/// libraries and the dyld shared cache, sysctl, mach lookups), deny all network,
/// and permit writes to exactly the one staged output file plus /dev/null.
///
/// `output` MUST already be canonicalized by the caller.
fn seatbelt_profile(output: &Path) -> String {
    let output = seatbelt_quote(&output.to_string_lossy());
    format!(
        "(version 1)\
         (deny default)\
         (allow process-fork)\
         (allow process-exec)\
         (allow sysctl-read)\
         (allow mach-lookup)\
         (allow file-read*)\
         (deny network*)\
         (allow file-write-data (literal \"/dev/null\"))\
         (allow file-write* (literal \"{output}\"))"
    )
}

/// Escape a path for embedding inside a Seatbelt string literal.
fn seatbelt_quote(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(dead_code))]
fn find_tool(candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn env_is_one(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| value == "1")
}

fn fail_closed_message(tool: &str, hint: &str) -> String {
    format!(
        "refusing to run the untrusted candidate worker without a sandbox: {tool} was not found \
         ({hint}). Local runs sandbox order() by default so a malicious pushed-back submission \
         cannot reach your network or files (issue #47). Install the sandbox, or set \
         {ALLOW_UNSANDBOXED_ENV}=1 to run the worker directly on your host at your own risk."
    )
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
        // In grader mode the opt-out is irrelevant: a missing/empty selection is
        // always a hard error.
        assert!(WorkerSandbox::select(true, None, true)
            .unwrap_err()
            .contains("must be exactly \"bubblewrap\""));
        assert!(WorkerSandbox::select(true, Some("".into()), true)
            .unwrap_err()
            .contains("must be exactly \"bubblewrap\""));
    }

    #[test]
    fn local_opt_out_yields_disabled_only_when_explicit() {
        // The explicit opt-out downgrades to direct execution...
        assert!(matches!(
            WorkerSandbox::select(false, None, true).unwrap(),
            WorkerSandbox::Disabled
        ));
        // ...but WITHOUT it, a local run never silently returns Disabled: it
        // either selects the host-native sandbox or fails closed.
        match WorkerSandbox::select(false, None, false) {
            Ok(WorkerSandbox::Disabled) => panic!("local default must not be unsandboxed"),
            Ok(_) => {} // native sandbox available on this host
            Err(message) => {
                assert!(message.contains(ALLOW_UNSANDBOXED_ENV));
                assert!(message.contains("#47"));
            }
        }
    }

    #[test]
    fn rejects_unknown_explicit_selection() {
        assert!(WorkerSandbox::select(false, Some("firejail".into()), false)
            .unwrap_err()
            .contains("must be unset or exactly \"bubblewrap\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_local_default_selects_seatbelt() {
        assert!(matches!(
            WorkerSandbox::select(false, None, false).unwrap(),
            WorkerSandbox::Seatbelt { .. }
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn seatbelt_command_denies_network_and_allows_only_the_output() {
        let dir = std::env::temp_dir().join(format!("ssi-seatbelt-command-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let candidate = dir.join("candidate");
        let input = dir.join("input");
        let output = dir.join("output");
        for path in [&candidate, &input, &output] {
            std::fs::write(path, b"x").unwrap();
        }
        let sandbox = WorkerSandbox::Seatbelt {
            executable: PathBuf::from("/usr/bin/sandbox-exec"),
        };
        let cmd = sandbox
            .worker_command(&candidate, WorkerInvocation::Order, &input, &output, &dir)
            .unwrap();
        assert_eq!(cmd.get_program(), OsStr::new("/usr/bin/sandbox-exec"));

        let args: Vec<String> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "-p");
        let profile = &args[1];
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(deny network*)"));

        // The write-allow literal must be the CANONICAL output path (so the
        // /var -> /private/var symlink does not defeat the match), and the
        // candidate/input/output paths follow the profile, canonicalized.
        let canonical_output = std::fs::canonicalize(&output).unwrap();
        assert!(profile.contains(&format!(
            "(allow file-write* (literal \"{}\"))",
            canonical_output.display()
        )));
        let canonical_candidate = std::fs::canonicalize(&candidate)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(args[2], canonical_candidate);
        assert_eq!(args.last().unwrap(), &canonical_output.to_string_lossy());

        // Environment is cleared, exactly like the other worker arms.
        assert!(cmd.get_envs().all(|(_, value)| value.is_none()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn seatbelt_profile_escapes_quotes_in_output_path() {
        let profile = seatbelt_profile(Path::new("/tmp/we\"ird/out.bin"));
        assert!(profile.contains("/tmp/we\\\"ird/out.bin"));
    }

    /// End-to-end proof that the Seatbelt profile does not break a genuine
    /// `order()` run: the built worker, launched under `sandbox-exec`, reads a
    /// real pattern and writes a valid permutation. Ignored unless
    /// `SSI_CANDIDATE_WORKER` names the built candidate binary.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires a built candidate worker (set SSI_CANDIDATE_WORKER)"]
    fn seatbelt_worker_produces_valid_permutation() {
        let candidate = std::env::var_os("SSI_CANDIDATE_WORKER")
            .map(PathBuf::from)
            .expect("SSI_CANDIDATE_WORKER must name the built candidate");
        let sandbox = WorkerSandbox::select(false, None, false).unwrap();
        assert!(matches!(sandbox, WorkerSandbox::Seatbelt { .. }));

        let dir = std::env::temp_dir().join(format!("ssi-seatbelt-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("pattern.bin");
        let output = dir.join("perm.bin");
        let cwd = dir.join("cwd");
        std::fs::create_dir(&cwd).unwrap();

        // A small path graph 0-1-2-3-4-5.
        let n = 6;
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        let pattern = ssi_scoring::Pattern::from_edges(n, &edges);
        ssi_worker_protocol::write_pattern(&input, &pattern).unwrap();
        std::fs::File::create(&output).unwrap();

        let mut cmd = sandbox
            .worker_command(&candidate, WorkerInvocation::Order, &input, &output, &cwd)
            .unwrap();
        let outcome = crate::watchdog::run_capped(
            &mut cmd,
            &crate::watchdog::CapConfig {
                time_cap: std::time::Duration::from_secs(2),
                poll: std::time::Duration::from_millis(10),
            },
        );
        assert_eq!(outcome, crate::watchdog::WorkerOutcome::Ok);

        let perm = ssi_worker_protocol::read_permutation(&output).unwrap();
        ssi_scoring::validate_permutation(&perm, n).expect("worker returned a valid bijection");
        std::fs::remove_dir_all(dir).unwrap();
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
        };
        let cmd = sandbox
            .worker_command(&candidate, WorkerInvocation::Order, &input, &output, &dir)
            .unwrap();
        let args: Vec<_> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let candidate = std::fs::canonicalize(candidate)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        assert_eq!(cmd.get_program(), OsStr::new("/test/bwrap"));
        assert!(args
            .windows(3)
            .any(|window| window == ["--ro-bind", &candidate, "/candidate-worker"]));
        assert!(args
            .windows(2)
            .any(|window| window == ["--", "/candidate-worker"]));
        assert!(!args.iter().any(|arg| arg == "--worker"));
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
