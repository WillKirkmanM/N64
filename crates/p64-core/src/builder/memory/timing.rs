#[derive(Default, Clone)]
pub struct MemoryTiming {
    pub cycles: Option<u64>,
    pub calculate_cycles_from_addr_size: bool,
}
