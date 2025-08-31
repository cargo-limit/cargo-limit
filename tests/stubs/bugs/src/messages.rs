#[derive(Default, Debug)]
pub struct Messages;

impl Messages {
    pub fn f() {
        non_existent(); // NOTE
    }
}
