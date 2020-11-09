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
