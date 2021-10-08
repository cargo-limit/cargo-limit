use crate::options::Options;
use anyhow::{format_err, Context, Result};
use atomic_enum::atomic_enum;
use getset::MutGetters;
use std::{
    env, fmt,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{atomic::Ordering, Arc},
    thread,
    time::Duration,
};

pub(crate) const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";

#[derive(Debug, MutGetters)]
pub struct CargoProcess {
    #[get_mut = "pub"]
    child: Child,
    state: Arc<AtomicState>,
}

#[atomic_enum]
#[derive(PartialEq)]
enum State {
    Running,
    KillTimerStarted,
    Killed,
}

impl CargoProcess {
    pub fn run(parsed_args: &Options) -> Result<Self> {
        let cargo_path = env::var(CARGO_ENV_VAR)
            .map(PathBuf::from)
            .ok()
            .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

        let error_text = failed_to_execute_error_text(&cargo_path);
        let child = Command::new(cargo_path)
            .args(parsed_args.all_args())
            .stdout(Stdio::piped())
            .spawn()
            .context(error_text)?;

        let state = Arc::new(AtomicState::new(State::Running));
        ctrlc::set_handler({
            let pid = child.id();
            let state = state.clone();
            move || {
                dbg!("ctrl+c");
                Self::kill(pid, state.clone());
                dbg!("killed on ctrl+c");
            }
        })?;

        Ok(Self { child, state })
    }

    pub fn kill_after_timeout<F: 'static>(&self, time_limit: Duration, after_kill: F)
    where
        F: Fn() + Send,
    {
        dbg!("trying to start kill timer");
        if self.can_start_kill_timer() {
            dbg!("kill timer has started");
            thread::spawn({
                let pid = self.child.id();
                let state = self.state.clone();
                move || {
                    thread::sleep(time_limit);
                    Self::kill(pid, state);
                    after_kill();
                }
            });
        }
    }

    fn kill(pid: u32, state: Arc<AtomicState>) {
        dbg!("trying to kill");
        if Self::can_kill(state) {
            dbg!("killing");
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

    fn can_start_kill_timer(&self) -> bool {
        self.state
            .compare_exchange(
                State::Running,
                State::KillTimerStarted,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn can_kill(state: Arc<AtomicState>) -> bool {
        state
            .compare_exchange(
                State::Running,
                State::Killed,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
            || state
                .compare_exchange(
                    State::KillTimerStarted,
                    State::Killed,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
    }
}

pub(crate) fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    format!("failed to execute {:?}", app)
}
