use crate::options::Options;
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

pub(crate) const CARGO_EXECUTABLE: &str = "";

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
