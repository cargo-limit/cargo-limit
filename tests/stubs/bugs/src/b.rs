use std::time::Duration;

#[derive(Debug, PartialEq)]
struct B {
    b: Duration,
}

impl Default for B {
    fn default() -> Self {
        Self {
            b: Some(Duration::from_secs(1)), // NOTE
        }
    }
}

impl B {
    fn f() {
        let mut result = B::default();
        let mut seconds = result
            .b
            .map(Duration::as_secs) // NOTE
            .unwrap_or(0);
        let duration = Duration::from_secs(seconds);
        result.b = if duration > Duration::from_secs(0) {
            Some(duration) // NOTE
        } else {
            None // NOTE
        };
    }
}
