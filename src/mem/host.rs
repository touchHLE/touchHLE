/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Cross-platform memory management wrappers using the host's system calls.

/// Cross-platform memory allocation using the host's system calls.
/// Returns an address aligned to the guest's 4KB page boundaries.
///
/// On hosts with a page size >= 4KB (all windows, and most unix likes), the
/// allocation will always be guest aligned.
///
/// On host's with a page size < 4KB this call will overallocate and return
/// a 4KB aligned address from the allocation, trimming the excess.
///
/// - The function returns a raw pointer to allocated memory.
///   The caller is responsible for managing that memory.
/// - The returned pointer must be freed using the corresponding
///   [`free_memory`] call.
#[cfg(windows)]
pub(super) unsafe fn allocate_memory(size: usize) -> std::io::Result<*mut core::ffi::c_void> {
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
pub(super) unsafe fn allocate_memory(size: usize) -> std::io::Result<*mut core::ffi::c_void> {
    use libc::{
        mmap, munmap, sysconf, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, _SC_PAGESIZE,
    };

    const PAGE_SIZE: usize = crate::mem::PAGE_SIZE as usize;

    let host_page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };

    let (ptr, allocation_size) = if host_page_size < PAGE_SIZE {
        // Overallocate in case system page size is less than PAGE_SIZE
        // so that we can find a PAGE_SIZE aligned start address
        let allocation_size = size + PAGE_SIZE;
        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                allocation_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        (ptr, allocation_size)
    } else {
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
        (ptr, size)
    };

    if ptr == libc::MAP_FAILED {
        return Err(std::io::Error::last_os_error());
    }

    if allocation_size == size {
        return Ok(ptr);
    }

    let ptr_addr = ptr as usize;
    let aligned_addr = (ptr_addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    let end_of_needed = aligned_addr + size;
    let end_of_allocation = ptr_addr + allocation_size;
    if end_of_needed < end_of_allocation {
        let res = unsafe {
            munmap(
                end_of_needed as *mut core::ffi::c_void,
                end_of_allocation - end_of_needed,
            )
        };
        if res == -1 {
            unsafe { munmap(ptr, allocation_size) };
            return Err(std::io::Error::last_os_error());
        }
    }

    if aligned_addr > ptr_addr {
        let res = unsafe { munmap(ptr, aligned_addr - ptr_addr) };
        if res == -1 {
            unsafe { munmap(ptr, end_of_needed - ptr_addr) };
            return Err(std::io::Error::last_os_error());
        }
    }

    assert_eq!(
        end_of_needed - aligned_addr,
        size,
        "Final mapped region should be exactly requested size bytes."
    );

    Ok(aligned_addr as *mut core::ffi::c_void)
}

/// Cross-platform memory free using the host's system calls.
///
/// # Safety
/// - The address and size should match parameters and result of the
///   [`allocate_memory`] call.
#[cfg(windows)]
pub(super) unsafe fn free_memory(
    address: *mut core::ffi::c_void,
    _size: usize,
) -> std::io::Result<()> {
    use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

    let res = unsafe { VirtualFree(address, 0, MEM_RELEASE) };

    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(unix)]
pub(super) unsafe fn free_memory(
    address: *mut core::ffi::c_void,
    size: usize,
) -> std::io::Result<()> {
    use libc::munmap;

    let res = unsafe { munmap(address, size) };

    if res == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
