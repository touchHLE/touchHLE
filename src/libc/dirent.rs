/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `dirent.h`

use crate::dyld::FunctionExports;
use crate::fs::GuestPath;
use crate::libc::errno::set_errno;
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::{export_c_func, impl_GuestRet_for_large_struct, Environment};
use std::collections::HashMap;

/// This is an opaque struct and doesn't necessary
/// corresponds the Apple's one
/// TODO: match struct sizes
#[allow(clippy::upper_case_acronyms)]
struct DIR {
    idx: usize,
}
unsafe impl SafeRead for DIR {}

// While early iOS is 32-bit system, underling file system uses 64-bit inodes!
const MAXPATHLEN: usize = 1024;

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C, packed)]
struct dirent {
    d_ino: u64,
    d_seekoff: u64,
    d_reclen: u16,
    d_namlen: u16,
    d_type: u8,
    d_name: [u8; MAXPATHLEN],
}
unsafe impl SafeRead for dirent {}
impl_GuestRet_for_large_struct!(dirent);

#[derive(Default)]
pub struct State {
    open_dirs: HashMap<MutPtr<DIR>, Vec<String>>,
    read_dirs: HashMap<MutPtr<DIR>, Vec<MutPtr<dirent>>>,
}
impl State {
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.dirent
    }
}

fn opendir(env: &mut Environment, filename: ConstPtr<u8>) -> MutPtr<DIR> {
    let path_string = env.mem.cstr_at_utf8(filename).unwrap().to_owned();
    log_dbg!("opendir: filename {}", path_string);
    let guest_path = GuestPath::new(&path_string);
    let is_dir = env.fs.is_dir(guest_path);
    if is_dir {
        let dir = env.mem.alloc_and_write(DIR { idx: 0 });
        log_dbg!("opendir: new DIR ptr: {:?}", dir);
        let iter = env.fs.enumerate(guest_path).unwrap();
        let vec = iter.map(|str| str.to_string()).collect();
        assert!(!State::get_mut(env).open_dirs.contains_key(&dir));
        State::get_mut(env).open_dirs.insert(dir, vec);
        assert!(!State::get_mut(env).read_dirs.contains_key(&dir));
        State::get_mut(env).read_dirs.insert(dir, Vec::new());
        dir
    } else {
        Ptr::null()
    }
}

// TODO: return '.' and '..' entries as well
fn readdir(env: &mut Environment, dirp: MutPtr<DIR>) -> MutPtr<dirent> {
    let mut dir = env.mem.read(dirp);
    let vec = env.libc_state.dirent.open_dirs.get(&dirp).unwrap();
    log_dbg!(
        "readdir: dirp {:?}, idx {}, entry '{:?}'",
        dirp,
        dir.idx,
        vec.get(dir.idx)
    );
    if let Some(str) = vec.get(dir.idx) {
        dir.idx += 1;
        env.mem.write(dirp, dir);

        let len = str.len();
        // TODO: fill other fields
        let mut dirent = dirent {
            d_ino: 0,
            d_seekoff: 0,
            d_reclen: 0,
            d_namlen: len as u16,
            d_type: 0,
            d_name: [b'\0'; MAXPATHLEN],
        };
        dirent.d_name[..len].copy_from_slice(str.as_bytes());
        let res = env.mem.alloc_and_write(dirent);
        env.libc_state
            .dirent
            .read_dirs
            .get_mut(&dirp)
            .unwrap()
            .push(res);
        res
    } else {
        Ptr::null()
    }
}

fn closedir(env: &mut Environment, dirp: MutPtr<DIR>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log_dbg!("closedir: dirp {:?}", dirp);
    if let Some(vec) = env.libc_state.dirent.read_dirs.remove(&dirp) {
        for dirent in vec {
            env.mem.free(dirent.cast());
        }
    }
    if env.libc_state.dirent.open_dirs.remove(&dirp).is_some() {
        // this avoid double free if closedir() is called twice
        env.mem.free(dirp.cast());
    }
    0 // Success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(opendir(_)),
    export_c_func!(readdir(_)),
    export_c_func!(closedir(_)),
];
