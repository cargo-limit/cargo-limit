use crate::options::Options;
use anyhow::{Context, Result};
use atomig::{Atom, Atomic};
use getset::MutGetters;
use std::{
    env, fmt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{atomic::Ordering, Arc},
    thread,
    time::Duration,
};

pub(crate) const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";

#[doc(hidden)]
pub const NO_EXIT_CODE: i32 = 127;

#[derive(Debug, MutGetters)]
pub struct CargoProcess {
    #[get_mut = "pub"]
    child: Child,
    state: Arc<Atomic<State>>,
}

#[derive(Atom, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum State {
    Running,
    KillTimerStarted,
    Killing,
    NotRunning,
    FailedToKill,
}

trait StateExt {
    fn try_start_killing(&self) -> bool;
    fn try_start_kill_timer(&self) -> bool;
    fn set_not_running(&self);
    fn force_set_not_running(&self);
    fn set_failed_to_kill(&self);
}

impl CargoProcess {
    pub fn run(options: &Options) -> Result<Self> {
        let cargo_path = env::var(CARGO_ENV_VAR)
            .map(PathBuf::from)
            .ok()
            .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

        let error_text = failed_to_execute_error_text(&cargo_path);
        let child = Command::new(cargo_path)
            .args(options.all_args())
            .stdout(Stdio::piped())
            .spawn()
            .context(error_text)?;

        let state = Arc::new(Atomic::new(State::Running));
        ctrlc::set_handler({
            let pid = child.id();
            let state = state.clone();
            move || {
                Self::kill(pid, state.clone());
            }
        })?;

        Ok(Self { child, state })
    }

    pub fn wait(&mut self) -> Result<i32> {
        let exit_status = self.child.wait()?;
        self.state.force_set_not_running();
        Ok(exit_status.code().unwrap_or(NO_EXIT_CODE))
    }

    pub fn wait_if_killing_is_in_progress(&self) -> State {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state == State::Killing {
                thread::yield_now();
            } else {
                break state;
            }
        }
    }

    pub fn kill_after_timeout(&self, time_limit: Duration) {
        if self.state.try_start_kill_timer() {
            thread::spawn({
                let pid = self.child.id();
                let state = self.state.clone();
                move || {
                    thread::sleep(time_limit);
                    Self::kill(pid, state);
                }
            });
        }
    }

    fn kill(pid: u32, state: Arc<Atomic<State>>) {
        if state.try_start_killing() {
            let success = {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGINT) == 0
                }

                #[cfg(windows)]
                {
                    use std::process::Output;
                    if let Ok(Output { stderr, .. }) = Command::new("taskkill")
                        .args(&["/PID", pid.to_string().as_str(), "/t"])
                        .output()
                    {
                        String::from_utf8_lossy(&stderr).starts_with("SUCCESS")
                    } else {
                        false
                    }
                }

                #[cfg(not(any(unix, windows)))]
                compile_error!("this platform is unsupported");
            };

            if success {
                state.set_not_running()
            } else {
                state.set_failed_to_kill()
            }
        }
    }
}

impl StateExt for Arc<Atomic<State>> {
    fn try_start_killing(&self) -> bool {
        self.compare_exchange(
            State::Running,
            State::Killing,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
            || self
                .compare_exchange(
                    State::KillTimerStarted,
                    State::Killing,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
    }

    fn try_start_kill_timer(&self) -> bool {
        self.compare_exchange(
            State::Running,
            State::KillTimerStarted,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
    }

    fn set_not_running(&self) {
        let _ = self.compare_exchange(
            State::Killing,
            State::NotRunning,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn force_set_not_running(&self) {
        self.store(State::NotRunning, Ordering::Release);
    }

    fn set_failed_to_kill(&self) {
        let _ = self.compare_exchange(
            State::Killing,
            State::FailedToKill,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

pub(crate) fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    format!("failed to execute {:?}", app)
}
