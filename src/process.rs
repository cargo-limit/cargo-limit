use std::{thread, time::Duration};

#[doc(hidden)]
pub fn kill(pid: u32) {
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
}

#[doc(hidden)]
pub fn kill_after_timeout(pid: u32, time_limit: Duration) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        thread::sleep(time_limit);
        kill(pid)
    })
}
