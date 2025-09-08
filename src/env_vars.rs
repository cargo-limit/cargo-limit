use const_format::concatcp;

pub const CARGO: &str = "CARGO";

pub const ASC: &str = concatcp!(CARGO, "_ASC");
pub const DEPS_WARN: &str = concatcp!(CARGO, "_DEPS_WARN");
pub const EDITOR: &str = concatcp!(CARGO, "_EDITOR");
pub const FORCE_WARN: &str = concatcp!(CARGO, "_FORCE_WARN");
pub const MSG_LIMIT: &str = concatcp!(CARGO, "_MSG_LIMIT");
pub const TIME_LIMIT: &str = concatcp!(CARGO, "_TIME_LIMIT");
