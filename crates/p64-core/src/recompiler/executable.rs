#![deny(unsafe_op_in_unsafe_fn)]
use std::ptr::NonNull;

const ENTRY_ALIGNMENT: usize = 16;

pub(super) struct ExecutablePage {
    pointer: NonNull<u8>,
    used: usize,
    sealed: bool,
}
// SAFETY: The mapping has unique ownership and no thread affinity. Mutation
// requires an exclusive borrow, and callable entries remain owned by the engine.
unsafe impl Send for ExecutablePage {}

impl ExecutablePage {
    pub(super) const SIZE: usize = 4096;
}

impl Drop for ExecutablePage {
    // SAFETY: Unique ownership releases the original allocation exactly once.
    fn drop(&mut self) {
        unsafe {
            executable_memory::free(self.pointer.as_ptr(), Self::SIZE);
        }
    }
}

#[cfg(all(target_arch = "x86_64", windows))]
mod executable_memory {
    use std::ffi::c_void;

    const MEM_COMMIT_RESERVE: u32 = 0x3000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_READWRITE: u32 = 4;
    const PAGE_EXECUTE_READ: u32 = 0x20;

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualAlloc(
            address: *mut c_void,
            size: usize,
            kind: u32,
            protection: u32,
        ) -> *mut c_void;
        fn VirtualProtect(address: *mut c_void, size: usize, protection: u32, old: *mut u32)
            -> i32;
        fn VirtualFree(address: *mut c_void, size: usize, kind: u32) -> i32;
        fn FlushInstructionCache(process: *mut c_void, address: *const c_void, size: usize) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
    }

    pub unsafe fn free(pointer: *mut u8, _: usize) {
        // SAFETY: Callers release the original allocation base exactly once.
        unsafe {
            VirtualFree(pointer.cast(), 0, MEM_RELEASE);
        }
    }
}

#[cfg(all(target_arch = "x86_64", unix))]
mod executable_memory {

    pub unsafe fn free(pointer: *mut u8, size: usize) {
        // SAFETY: Callers release the original allocation range exactly once.
        unsafe {
            libc::munmap(pointer.cast(), size);
        }
    }
}

#[cfg(all(target_arch = "x86_64", not(any(windows, unix))))]
mod executable_memory {

    pub unsafe fn free(_: *mut u8, _: usize) {}
}
