use super::*;

#[derive(Default, PartialEq)]
pub enum State {
    #[default]
    Step,
    Take,
    NotTaken,
    DelaySlotTaken,
    DelaySlotNotTaken,
    Discard,
    Exception,
}

#[derive(Default)]
pub struct BranchState {
    pub state: State,
    pub pc: u64,
}

pub struct Cpu {
    pub mmu: mmu::Mmu,
    pub fpu: Fpu,
    pub rcp: rcp::Rcp,
    pub branch_state: BranchState,
    pub gpr: [u64; 32],
    pub pc: u64,
    pub pc_phys: u64,
    pub lo: u64,
    pub hi: u64,
    pub running: u8,
    pub llbit: bool,
    pub clock_rate: u64,
    pub instrs: [fn(&mut N64, u32); 64],
    pub special_instrs: [fn(&mut N64, u32); 64],
    pub regimm_instrs: [fn(&mut N64, u32); 32],
    pub events: [events::Event; EventType::Count as usize],
    pub next_event_count: u64,
    pub next_event: usize,
}

impl Default for Cpu {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        info!("Initialising CPU...");

        let cpu = Cpu {
            mmu: Mmu {
                regs: Registers::<u64>::new().with_size(MmuRegister::RegsCount as usize),
                reg_write_masks: [0; MmuRegister::RegsCount as usize],
                reg_latch: 0,
                instrs: MMU_INSTRUCTIONS,
                instrs2: MMU_SECONDARY_INSTRUCTIONS,
                tlb_lut_w: vec![
                    tlb::TlbLookupTable {
                        address: 0,
                        cached: false,
                    };
                    0x100000
                ],
                tlb_lut_r: vec![
                    tlb::TlbLookupTable {
                        address: 0,
                        cached: false,
                    };
                    0x100000
                ],
                tlb_entries: [tlb::TlbEntry::default(); 32],
                // ..Default::default()
            },
            fpu: Fpu::default(),
            rcp: Rcp::default(),
            branch_state: BranchState {
                state: State::Step,
                pc: 0,
            },
            gpr: [0; 32],
            clock_rate: 0,
            pc: 0xBFC00000,
            pc_phys: 0,
            llbit: false,
            lo: 0,
            hi: 0,
            running: 0,
            instrs: [default_instruction; 64],
            special_instrs: [default_instruction; 64],
            regimm_instrs: [default_instruction; 32],
            events: [events::Event {
                enabled: false,
                count: u64::MAX,
                handler: events::dummy_event,
            }; EventType::Count as usize],
            next_event_count: u64::MAX,
            next_event: 0,
        };

        info!("CPU has Initialised");

        cpu
    }
}
