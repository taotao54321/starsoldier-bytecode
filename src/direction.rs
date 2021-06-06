#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Direction(u8);

impl Direction {
    pub fn new(idx: u8) -> Self {
        assert!((0..=0x3F).contains(&idx));
        Self(idx)
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn displacement_object(self) -> (i8, i8) {
        todo!();
    }

    pub fn displacement_bullet(self) -> (i8, i8) {
        todo!();
    }
}
