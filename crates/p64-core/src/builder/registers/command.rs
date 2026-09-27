#[derive(Clone, Copy)]
pub struct FlagCommand {
    clear: u32,
    set: u32,
    flag: u32,
}

impl FlagCommand {
    pub const fn new(clear: u32, set: u32, flag: u32) -> Self {
        Self { clear, set, flag }
    }

    #[inline]
    pub fn apply(self, state: &mut u32, command: u32) {
        if command & self.clear != 0 {
            *state &= !self.flag;
        }

        if command & self.set != 0 {
            *state |= self.flag;
        }
    }
}
