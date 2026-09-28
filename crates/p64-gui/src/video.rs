use crate::ui;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

const RDRAM_ALIGNMENT: usize = 64 * 1024;
const DMEM_SIZE: usize = 4096;
static RENDERER_LOCK: Mutex<()> = Mutex::new(());

fn lock_renderer() -> MutexGuard<'static, ()> {
    RENDERER_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct Renderer {
    _thread: PhantomData<Rc<()>>,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _lock = lock_renderer();
        unsafe { vk_close() };
    }
}

extern "C" {
    fn vk_init(rdram_ptr: usize, rdram_size: u32, fullscreen: u8) -> u8;
    fn vk_close();
    fn vk_is_active() -> u8;
    fn vk_set_vsync(enabled: u8);
    fn vk_set_unthrottled(enabled: u8);
    fn set_sdl_window(window: usize);
    fn rdp_update_screen() -> u8;
    fn rdp_set_vi_register(reg: u32, value: u32);
    fn rdp_process_commands(dpc_regs: &mut [u32; 8], sp_dmem: *const u8, dmem_size: usize) -> u64;
    fn full_sync();
    fn set_upscaling_1x();
    fn set_upscaling_2x();
    fn set_upscaling_4x();
    fn set_upscaling_8x();
}

pub fn set_unthrottled(enabled: bool) {
    let _lock = lock_renderer();
    unsafe {
        vk_set_unthrottled(enabled as u8);
    }
}

mod viewport;
pub use viewport::ViewportData;

extern "C" {
    fn calculate_viewport_values(width: i32, height: i32) -> ViewportData;
}

pub fn set_viewport(width: i32, height: i32) -> ViewportData {
    let _lock = lock_renderer();
    unsafe { calculate_viewport_values(width, height) }
}

/// # Safety
/// RDRAM must remain allocated at this aligned address, with exclusive host access,
/// until the returned renderer is dropped. Its SDL window must also remain alive.
pub unsafe fn start_window(
    ui: &mut ui::Ui,
    rdram_ptr: *mut u8,
    rdram_size: usize,
    width: u32,
    height: u32,
    fullscreen: bool,
) -> Result<Renderer, String> {
    let _lock = lock_renderer();
    if unsafe { vk_is_active() } != 0 {
        return Err("A renderer is already active".to_owned());
    }
    let rdram_size = validate_rdram(rdram_ptr, rdram_size)?;
    let mut builder = ui
        .sdl_context
        .video_subsystem
        .as_ref()
        .ok_or_else(|| "SDL video subsystem unavailable".to_owned())?
        .window("p64", width, height);
    builder.vulkan();
    builder.position_centered().vulkan().resizable();

    if fullscreen {
        builder.fullscreen_desktop();
    }

    let window = builder.build().map_err(|error| error.to_string())?;

    unsafe {
        set_sdl_window(window.raw() as usize);
        let status = vk_init(rdram_ptr as usize, rdram_size, fullscreen as u8);
        if status != 0 {
            vk_close();
            return Err(format!(
                "Vulkan renderer initialization failed (stage {status})"
            ));
        }
    }
    ui.window_audio.window = Some(window);
    Ok(Renderer {
        _thread: PhantomData,
    })
}

fn validate_rdram(pointer: *mut u8, length: usize) -> Result<u32, String> {
    if pointer.is_null() || !pointer.addr().is_multiple_of(RDRAM_ALIGNMENT) || length == 0 {
        return Err("RDRAM must have a nonzero size and 64 KiB aligned address".to_owned());
    }
    u32::try_from(length).map_err(|_| "RDRAM exceeds the renderer address space".to_owned())
}

pub fn set_vsync(enabled: bool) {
    let _lock = lock_renderer();
    unsafe { vk_set_vsync(enabled as u8) };
}

pub fn update_screen() -> u8 {
    let _lock = lock_renderer();
    unsafe { rdp_update_screen() }
}

pub fn set_register(reg: u32, value: u32) {
    let _lock = lock_renderer();
    unsafe {
        rdp_set_vi_register(reg, value);
    }
}

pub fn process_rdp_list(dpc_regs: &mut [u32; 8], sp_dmem: &mut [u8]) -> u64 {
    let _lock = lock_renderer();
    if sp_dmem.len() < DMEM_SIZE {
        return 0;
    }
    unsafe { rdp_process_commands(dpc_regs, sp_dmem.as_ptr(), sp_dmem.len()) }
}

pub fn rdp_full_sync() {
    let _lock = lock_renderer();
    unsafe {
        full_sync();
    }
}

pub fn enable_1x_upscaling() {
    let _lock = lock_renderer();
    unsafe {
        set_upscaling_1x();
    }
}

pub fn enable_2x_upscaling() {
    let _lock = lock_renderer();
    unsafe {
        set_upscaling_2x();
    }
}

pub fn enable_4x_upscaling() {
    let _lock = lock_renderer();
    unsafe {
        set_upscaling_4x();
    }
}

pub fn enable_8x_upscaling() {
    let _lock = lock_renderer();
    unsafe { set_upscaling_8x() }
}

pub fn set_upscaling(scale: u8) {
    match scale {
        1 => enable_1x_upscaling(),
        2 => enable_2x_upscaling(),
        4 => enable_4x_upscaling(),
        8 => enable_8x_upscaling(),
        _ => {}
    }
}
