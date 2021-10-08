use crate::options::Options;
use anyhow::{format_err, Context, Result};
use getset::MutGetters;
use std::{
    env, fmt,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

pub(crate) const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";

#[derive(Debug, MutGetters)]
pub struct CargoProcess {
    #[get_mut = "pub"]
    child: Child,
}

impl CargoProcess {
    pub fn run(parsed_args: &Options) -> Result<Self> {
        let cargo_path = env::var(CARGO_ENV_VAR)
            .map(PathBuf::from)
            .ok()
            .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

        let error_text = failed_to_execute_error_text(&cargo_path);
        let mut child = Command::new(cargo_path) // TODO: extract to InterruptableCommand with Mutex<Running | KillTimerStarted | Killed>, move to process.rs
            .args(parsed_args.all_args())
            .stdout(Stdio::piped())
            .spawn()
            .context(error_text)?;

        // TODO: rename
        let pid = child.id();
        ctrlc::set_handler(move || {
            // TODO: check child_killed
            Self::kill(pid);
        })?;

        Ok(Self { child })
    }

    pub fn wait_in_background_and_kill<F: 'static>(&self, time_limit: Duration, after_kill: F)
    where
        F: Fn() + Send,
    {
        let pid = self.child.id();
        thread::spawn(move || {
            thread::sleep(time_limit);
            Self::kill(pid); // TODO: check shared atomic bool child_killed
            after_kill();
        });
    }

    fn kill(pid: u32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGINT);
        }

        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(&["/PID", pid.to_string().as_str(), "/t"])
                .output();
        }

        #[cfg(not(any(unix, windows)))]
        compile_error!("this platform is unsupported");
    }
}

pub(crate) fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    format!("failed to execute {:?}", app)
}
