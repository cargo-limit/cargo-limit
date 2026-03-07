unsafe extern "C" {
    fn force_link_error();
}

pub fn trigger() {
    unsafe { force_link_error() }
}

fn main() {
    trigger();
}
