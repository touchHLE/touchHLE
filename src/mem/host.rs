/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Cross-platform memory allocation wrappers using the host's system calls.
//!
//! Memory allocated using these methods will be page aligned according to
//! the host system as well as lazily initialized.
//!
//! # Safety
//!
//! The allocation wrappers return raw pointers to memory. The caller should
//! ensure proper management and access.

#[cfg(windows)]
pub(crate) fn allocate_memory(size: usize) -> std::io::Result<*mut core::ffi::c_void> {
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE,
    };

    let ptr = unsafe {
        VirtualAlloc(
            std::ptr::null(),
            size,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        )
    };

    if ptr.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(ptr)
}

#[cfg(unix)]
pub(crate) fn allocate_memory(size: usize) -> std::io::Result<*mut core::ffi::c_void> {
    use libc::{mmap, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};

    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return Err(std::io::Error::last_os_error());
    }
    Ok(ptr)
}

#[cfg(windows)]
pub(crate) fn free_memory(address: *mut core::ffi::c_void, _size: usize) -> std::io::Result<()> {
    use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

    let res = unsafe { VirtualFree(address, 0, MEM_RELEASE) };

    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(unix)]
pub(crate) fn free_memory(address: *mut core::ffi::c_void, size: usize) -> std::io::Result<()> {
    use libc::munmap;

    let res = unsafe { munmap(address, size) };

    if res == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
