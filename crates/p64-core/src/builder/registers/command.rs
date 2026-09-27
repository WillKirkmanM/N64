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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_wins_over_clear_without_touching_other_flags() {
        let control = FlagCommand::new(1, 2, 8);

        for (command, expected) in [(0, 0x18), (1, 0x10), (2, 0x18), (3, 0x18)] {
            let mut state = 0x18;
            control.apply(&mut state, command);

            assert_eq!(state, expected);
        }
    }
}
