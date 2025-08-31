#[derive(Default, Debug)]
pub struct Messages {
    pub child_killed: bool,
}

impl Messages {
    pub fn f() {
        non_existent(); // NOTE
    }

    pub fn merge(&mut self, other: Self) {
        todo!()
    }
}
