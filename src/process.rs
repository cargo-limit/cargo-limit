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

    #[cfg(not(any(unix, windows)))]
    compile_error!("this platform is unsupported");
}

#[doc(hidden)]
pub fn wait_in_background_and_kill_and_print_empty_line(pid: u32, time_limit: Duration) {
    thread::spawn(move || {
        thread::sleep(time_limit);
        kill(pid);
        println!();
    });
}
