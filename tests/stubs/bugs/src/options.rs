use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct Options {
    pub time_limit_after_error: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            time_limit_after_error: Some(Duration::from_secs(1)), // NOTE
        }
    }
}

impl Options {
    fn f() {
        let mut result = Self::default();
        let mut seconds = result
            .time_limit_after_error
            .map(Duration::as_secs) // NOTE
            .unwrap_or(0);
        let duration = Duration::from_secs(seconds);
        result.time_limit_after_error = if duration > Duration::from_secs(0) {
            Some(duration) // NOTE
        } else {
            None // NOTE
        };
    }
}
