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
    pub(super) fn new() -> Option<Self> {
        // SAFETY: The platform allocator returns a fresh writable mapping of
        // exactly SIZE bytes, or null. Drop releases each successful allocation.
        let pointer = NonNull::new(unsafe { executable_memory::allocate(Self::SIZE) })?;

        Some(Self {
            pointer,
            used: 0,
            sealed: false,
        })
    }

    pub(super) fn fits(&self, size: usize) -> bool {
        size <= Self::SIZE.saturating_sub(self.used)
    }

    pub(super) fn insert(&mut self, code: &[u8]) -> Option<*mut u8> {
        // SAFETY: This exclusive borrow owns the entire live mapping; callers
        // cannot execute an entry while the engine is modifying its page.
        unsafe {
            if !executable_memory::protect(self.pointer.as_ptr(), Self::SIZE, false) {
                return None;
            }
        }
        self.sealed = false;
        let entry = self.insert_unsealed(code)?;

        if !self.seal() {
            return None;
        }

        Some(entry)
    }

    pub(super) fn insert_unsealed(&mut self, code: &[u8]) -> Option<*mut u8> {
        if self.sealed || code.is_empty() || !self.fits(code.len()) {
            return None;
        }
        // SAFETY: fits() bounds the destination inside our writable mapping,
        // and code is separately owned emitter/artifact memory, never this page.
        let entry = unsafe { self.pointer.as_ptr().add(self.used) };
        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), entry, code.len());
        }
        self.used = crate::bits::align_up(self.used + code.len(), ENTRY_ALIGNMENT);

        Some(entry)
    }

    #[cfg(test)]
    pub(super) fn is_sealed(&self) -> bool {
        self.sealed
    }

    pub(super) fn seal(&mut self) -> bool {
        if self.sealed {
            return true;
        }
        // SAFETY: The pointer and SIZE describe the mapping owned by this page.
        self.sealed =
            unsafe { executable_memory::protect(self.pointer.as_ptr(), Self::SIZE, true) };

        self.sealed
    }
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
    pub unsafe fn allocate(size: usize) -> *mut u8 {
        // SAFETY: Null requests a new OS-owned mapping, committed read/write.
        unsafe {
            VirtualAlloc(
                std::ptr::null_mut(),
                size,
                MEM_COMMIT_RESERVE,
                PAGE_READWRITE,
            )
            .cast()
        }
    }

    pub unsafe fn protect(pointer: *mut u8, size: usize, executable: bool) -> bool {
        let mut old = 0;
        // SAFETY: Callers pass the entire live allocation and its original size.
        let result = unsafe {
            VirtualProtect(
                pointer.cast(),
                size,
                if executable {
                    PAGE_EXECUTE_READ
                } else {
                    PAGE_READWRITE
                },
                &mut old,
            ) != 0
        };

        if executable && result {
            // SAFETY: The successful protection change leaves this range live.
            unsafe {
                FlushInstructionCache(GetCurrentProcess(), pointer.cast(), size);
            }
        }

        result
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
    pub unsafe fn allocate(size: usize) -> *mut u8 {
        // SAFETY: Anonymous private mapping has no backing descriptor or offset.
        let pointer = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0 as libc::off_t,
            )
        };

        if pointer == libc::MAP_FAILED {
            std::ptr::null_mut()
        } else {
            pointer.cast()
        }
    }

    pub unsafe fn protect(pointer: *mut u8, size: usize, executable: bool) -> bool {
        let protection = libc::PROT_READ
            | if executable {
                libc::PROT_EXEC
            } else {
                libc::PROT_WRITE
            };
        // SAFETY: Callers pass the entire live allocation and its original size.
        unsafe { libc::mprotect(pointer.cast(), size, protection) == 0 }
    }

    pub unsafe fn free(pointer: *mut u8, size: usize) {
        // SAFETY: Callers release the original allocation range exactly once.
        unsafe {
            libc::munmap(pointer.cast(), size);
        }
    }
}

#[cfg(all(target_arch = "x86_64", not(any(windows, unix))))]
mod executable_memory {
    pub unsafe fn allocate(_: usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    pub unsafe fn protect(_: *mut u8, _: usize, _: bool) -> bool {
        false
    }

    pub unsafe fn free(_: *mut u8, _: usize) {}
}

#[cfg(test)]
mod tests;
