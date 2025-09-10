use crate::{
    env_vars,
    io::Buffers,
    options::{COLOR_ALWAYS, COLOR_NEVER, Options},
};
use anyhow::{Context, Result};
use atomig::{Atom, Atomic};
use std::{
    env, fmt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, atomic::Ordering},
    thread,
    time::Duration,
};

pub const CARGO_EXECUTABLE: &str = "cargo";

#[doc(hidden)]
pub const NO_EXIT_CODE: i32 = 127;

#[derive(Debug)]
pub struct CargoProcess {
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
    fn try_set_killing(&self) -> bool;
    fn try_set_start_kill_timer(&self) -> bool;
    fn set_not_running(&self);
    fn force_set_not_running(&self);
    fn set_failed_to_kill(&self);
    fn transit(&self, current: State, new: State) -> bool;
}

impl CargoProcess {
    pub fn run(options: &Options) -> Result<Self> {
        let cargo_path = env::var(env_vars::CARGO)
            .map(PathBuf::from)
            .ok()
            .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

        let error_text = failed_to_execute_error_text(&cargo_path);
        let envs = if options.color == COLOR_NEVER {
            env::var(env_vars::TERM_COLOR)
                .ok()
                .map(|value| (env_vars::TERM_COLOR, value))
                .into_iter()
                .collect()
        } else {
            vec![(
                env_vars::TERM_COLOR,
                env::var(env_vars::TERM_COLOR)
                    .ok()
                    .unwrap_or(COLOR_ALWAYS.to_string()),
            )]
        };
        let child = Command::new(cargo_path)
            .envs(envs)
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

    pub fn buffers(&mut self) -> Result<Buffers> {
        Buffers::new(&mut self.child)
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
        if self.state.try_set_start_kill_timer() {
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
        if state.try_set_killing() {
            let success = {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGINT) == 0
                }

                #[cfg(windows)]
                {
                    use std::process::Output;
                    if let Ok(Output { stderr, .. }) = Command::new("taskkill")
                        .args(["/PID", pid.to_string().as_str(), "/t"])
                        .output()
                    {
                        stderr.starts_with(b"SUCCESS")
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
    fn try_set_killing(&self) -> bool {
        self.transit(State::Running, State::Killing)
            || self.transit(State::KillTimerStarted, State::Killing)
    }

    fn try_set_start_kill_timer(&self) -> bool {
        self.transit(State::Running, State::KillTimerStarted)
    }

    fn set_not_running(&self) {
        let _ = self.transit(State::Killing, State::NotRunning);
    }

    fn force_set_not_running(&self) {
        self.store(State::NotRunning, Ordering::Release);
    }

    fn set_failed_to_kill(&self) {
        let _ = self.transit(State::Killing, State::FailedToKill);
    }

    fn transit(&self, current: State, new: State) -> bool {
        self.compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

pub(crate) fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    format!("failed to execute {app:?}")
}
