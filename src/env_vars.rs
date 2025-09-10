use const_format::concatcp;

pub const CARGO: &str = "CARGO";
pub const RUSTFLAGS: &str = "RUSTFLAGS";
pub const TERM_COLOR: &str = concatcp!(CARGO, "_TERM_COLOR");

pub const ASC: &str = concatcp!(CARGO, "_ASC");
pub const DEPS_WARN: &str = concatcp!(CARGO, "_DEPS_WARN");
pub const EDITOR: &str = concatcp!(CARGO, "_EDITOR");
pub const FORCE_WARN: &str = concatcp!(CARGO, "_FORCE_WARN");
pub const MSG_LIMIT: &str = concatcp!(CARGO, "_MSG_LIMIT");
pub const TIME_LIMIT: &str = concatcp!(CARGO, "_TIME_LIMIT");

pub const USER: &str = {
    #[cfg(unix)]
    {
        "USER"
    }

    #[cfg(windows)]
    {
        "USERNAME"
    }

    #[cfg(not(any(unix, windows)))]
    {
        compile_error!("this platform is unsupported")
    }
};
