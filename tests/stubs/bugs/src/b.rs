use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Options {
    a: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            a: Some(Duration::from_secs(1)), // NOTE
        }
    }
}

impl Options {
    fn f() {
        let mut result = Self::default();
        let mut seconds = result
            .a
            .map(Duration::as_secs) // NOTE
            .unwrap_or(0);
        let duration = Duration::from_secs(seconds);
        result.a = if duration > Duration::from_secs(0) {
            Some(duration) // NOTE
        } else {
            None // NOTE
        };
    }
}
