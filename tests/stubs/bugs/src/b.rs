use std::time::Duration;

#[derive(Debug, PartialEq)]
struct B {
    b: Duration,
}

impl Default for B {
    fn default() -> Self {
        Self {
            b: Some(Duration::from_secs(1)),
        }
    }
}

impl B {
    fn f() {
        let b = B::default().b.map(Duration::as_secs).unwrap_or(0);
        let _ = Duration::from_secs(b);
        result.b = None;
    }
}
