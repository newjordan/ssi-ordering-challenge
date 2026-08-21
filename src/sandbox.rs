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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerInvocation {
    Order,
    SandboxSelfCheck,
    SandboxTimeoutSelfCheck,
}

impl WorkerInvocation {
    fn flag(self) -> Option<&'static str> {
        match self {
            Self::Order => None,
            Self::SandboxSelfCheck => Some("--sandbox-self-check"),
            Self::SandboxTimeoutSelfCheck => Some("--sandbox-timeout-self-check"),
        }
    }
}

#[derive(Debug)]
pub enum WorkerSandbox {
    Disabled,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Bubblewrap {
        executable: PathBuf,
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
            Ok(Self::Bubblewrap { executable })
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
    ) -> Result<Command, String> {
        match self {
            Self::Disabled => {
                let mut cmd = Command::new(candidate_exe);
                if let Some(flag) = invocation.flag() {
                    cmd.arg(flag);
                }
                cmd.arg(input)
                    .arg(output)
                    .env_clear()
                    .current_dir(local_cwd);
                // This hook exists only for the local watchdog regression test.
                // Grader workers receive no environment entries at all.
                if let Ok(sleep_ms) = std::env::var("SSI_TEST_SLEEP_MS") {
                    cmd.env("SSI_TEST_SLEEP_MS", sleep_ms);
                }
                null_stdio(&mut cmd);
                Ok(cmd)
            }
            Self::Bubblewrap { executable } => {
                let candidate_exe = canonical_file(candidate_exe, "candidate executable")?;
                let input = canonical_file(input, "worker input")?;
                let output = canonical_file(output, "worker output")?;

                let mut cmd = Command::new(executable);
                // NOTE: deliberately NO `--new-session`. The watchdog enforces
                // the time cap by SIGKILLing the worker's whole process group
                // (`kill -KILL -<pgid>`, see watchdog::run_capped). `--new-session`
                // would setsid() the sandboxed processes into their own session,
                // so that group-kill could never reach them and the watchdog's
                // `child.wait()` would hang while the candidate + any descendant
                // survived the cap. `--die-with-parent` only fires when the harness
                // (bwrap's parent) dies, not when bwrap is signalled, so it does not
                // cover the timeout path. Keeping the sandbox in bwrap's process
                // group lets one group-kill reap bwrap, the PID-1 candidate, and its
                // descendants together; killing PID 1 collapses the namespace.
                // `--new-session` guards against TIOCSTI tty injection, but the
                // worker has /dev/null stdio and no controlling terminal, so there
                // is nothing to inject into here.
                cmd.env_clear().args([
                    "--unshare-pid",
                    "--unshare-net",
                    "--unshare-ipc",
                    "--unshare-uts",
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
