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

#[derive(Debug)]
pub enum WorkerSandbox {
    Disabled,
    Bubblewrap { executable: PathBuf },
}

impl WorkerSandbox {
    /// Select the sandbox from the trusted parent's environment.
    ///
    /// Local development remains unchanged when `SSI_GRADER_SANDBOX` is unset.
    /// Any non-empty value other than `bubblewrap` is rejected, and requested
    /// grader isolation fails closed if the platform or binary is unsuitable.
    pub fn from_env() -> Result<Self, String> {
        let Some(value) = std::env::var_os(MODE_ENV) else {
            return Ok(Self::Disabled);
        };
        if value.is_empty() {
            return Ok(Self::Disabled);
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
                    "grader parent is running as root; refusing to launch untrusted code".to_string(),
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
        harness_exe: &Path,
        worker_flag: &str,
        input: &Path,
        output: &Path,
        local_cwd: &Path,
    ) -> Result<Command, String> {
        match self {
            Self::Disabled => {
                let mut cmd = Command::new(harness_exe);
                cmd.arg(worker_flag)
                    .arg(input)
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
                let harness_exe = canonical_file(harness_exe, "harness executable")?;
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
                    .arg(&harness_exe)
                    .arg("/worker")
                    .arg("--ro-bind")
                    .arg(&input)
                    .arg("/input/pattern.bin")
                    .arg("--bind")
                    .arg(&output)
                    .arg("/output/permutation.bin")
                    .args(["--chdir", "/", "--", "/worker", worker_flag])
                    .args(["/input/pattern.bin", "/output/permutation.bin"]);
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
        let mut cmd = sandbox
            .worker_command(
                Path::new("/harness"),
                "--worker",
                Path::new("/pattern"),
                Path::new("/permutation"),
                Path::new("/empty"),
            )
            .unwrap();

        assert_eq!(cmd.get_program(), OsStr::new("/harness"));
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            [
                OsStr::new("--worker"),
                OsStr::new("/pattern"),
                OsStr::new("/permutation")
            ]
        );
        assert!(cmd.get_envs().all(|(_, value)| value.is_none()));
        assert_eq!(cmd.get_current_dir(), Some(Path::new("/empty")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_effective_uid_not_real_uid() {
        let status = "Name:\ttest\nUid:\t1000\t1001\t1002\t1003\n";
        assert_eq!(parse_effective_uid(status), Some(1001));
        assert_eq!(parse_effective_uid("Name:\ttest\n"), None);
    }
}
