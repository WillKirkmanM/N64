use cartridge::{load_rom_save, write_cartridge_to_memory};
use cpu::start_cpu;
use memory::initialise_memory_map;
use p64_gui::ui::load_saves;
use p64_gui::video::start_window;
use pif::set_cartridge_region;
use rdram::get_rdram_shared_pointer;
use rom::try_read_rom;
use rsp::start_rsp;
use video::set_cpu_clock_speed;

pub mod aot;
pub mod audio;
mod bits;
pub mod builder;
pub mod cache;
pub mod cartridge;
pub mod cpu;
pub mod events;
pub mod exceptions;
pub mod fpu;
pub mod is_viewer;
pub mod memory;
pub mod mempak;
pub mod mips;
pub mod mmu;
mod opcode;
pub mod peripheral;
pub mod pif;
pub mod rcp;
pub mod rdp;
pub mod rdram;
pub mod recompiler;
pub mod rom;
pub mod rsp;
mod rsp_vector;
pub mod serial;
pub mod sram;
pub mod tlb;
pub mod video;

use std::time::Instant;

use tracing::info;

pub fn run_game(
    file_path: &std::path::Path,
    device: &mut N64,
    width: u32,
    height: u32,
    fullscreen: bool,
) -> Result<(), String> {
    let rom = try_read_rom(file_path)
        .map_err(|error| format!("Read ROM {}: {error}", file_path.display()))?;
    write_cartridge_to_memory(device, rom);

    let start_load_saves = Instant::now();
    load_saves(&mut device.ui).map_err(|error| format!("Load game saves: {error}"))?;
    info!("Load saves took: {:?}", start_load_saves.elapsed());

    let (rdram_ptr, rdram_size) = get_rdram_shared_pointer(device);

    let start_video = Instant::now();
    // SAFETY: The machine retains the aligned RDRAM allocation and SDL window
    // through CPU execution; renderer teardown precedes their destruction.
    let renderer = unsafe {
        start_window(
            &mut device.ui,
            rdram_ptr,
            rdram_size,
            width,
            height,
            fullscreen,
        )?
    };
    let duration_video = start_video.elapsed();
    info!("Video init took: {:?}", duration_video);

    set_cartridge_region(device);

    let start_memory = Instant::now();
    initialise_memory_map(device);
    let duration_memory = start_memory.elapsed();
    info!("Memory map init took: {:?}", duration_memory);

    start_rsp(device);
    set_cpu_clock_speed(device);
    start_cpu(device);

    let start_load_rom_save = Instant::now();
    load_rom_save(device);
    let duration_load_rom_save = start_load_rom_save.elapsed();
    info!("Load ROM save took: {:?}", duration_load_rom_save);

    cpu::run(device);
    drop(renderer);
    device
        .ui
        .flush()
        .map_err(|error| format!("Flush saves/configuration: {error}"))
}

mod machine;
pub use machine::N64;
