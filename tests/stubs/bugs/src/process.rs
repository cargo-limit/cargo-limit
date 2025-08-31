use crate::{io::Buffers, options::Options};
use anyhow::{Context, Result};
use atomig::{Atom, Atomic};
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
        todo!()
    }

    pub fn buffers(&mut self) -> Result<Buffers> {
        todo!()
    }

    pub fn wait(&mut self) -> Result<i32> {
        todo!()
    }

    pub fn wait_if_killing_is_in_progress(&self) -> State {
        todo!()
    }

    pub fn kill_after_timeout(&self, time_limit: Duration) {
        todo!()
    }

    fn kill(pid: u32, state: Arc<Atomic<State>>) {
        todo!()
    }
}

impl StateExt for Arc<Atomic<State>> {
    fn try_set_killing(&self) -> bool {
        todo!()
    }

    fn try_set_start_kill_timer(&self) -> bool {
        todo!()
    }

    fn set_not_running(&self) {
        todo!()
    }

    fn force_set_not_running(&self) {
        todo!()
    }

    fn set_failed_to_kill(&self) {
        todo!()
    }

    fn transit(&self, current: State, new: State) -> bool {
        todo!()
    }
}

pub(crate) fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    todo!()
}
