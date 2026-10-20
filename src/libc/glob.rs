/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! glob? glob. glob-glob

use super::dirent::{closedir, dirent, opendir, readdir, DIR};
use super::fnmatch::fnmatch;
use super::string::strlen;
use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, SafeRead};
use crate::Environment;

// Our internal type.
type GlobFlagType = i32;
const GLOB_DOOFFS: GlobFlagType = 0x2;
const GLOB_NOSORT: GlobFlagType = 0x20;
const GLOB_MAGCHAR: GlobFlagType = 0x100;
const GLOB_NOESCAPE: GlobFlagType = 0x2000;

const GLOB_NOMATCH: i32 = -3;

#[repr(C, packed)]
struct glob_t {
    gl_pathc: GuestUSize,
    gl_matchc: i32,
    gl_offs: GuestUSize,
    gl_flags: i32,
    gl_pathv: MutPtr<MutPtr<u8>>,
    gl_errfunc: GuestFunction,  // TODO
    gl_closedir: GuestFunction, // TODO
    gl_readdir: GuestFunction,  // TODO
    gl_opendir: GuestFunction,  // TODO
    gl_lstat: GuestFunction,    // TODO
    gl_stat: GuestFunction,     // TODO
}
unsafe impl SafeRead for glob_t {}

fn glob(
    env: &mut Environment,
    pattern: ConstPtr<u8>,
    flags: GlobFlagType,
    err_func: GuestFunction,
    pglob: MutPtr<glob_t>,
) -> i32 {
    let pattern_str = env.mem.cstr_at_utf8(pattern);
    log_dbg!(
        "glob({:?}, {}, {:?}, {:?})",
        pattern_str,
        flags,
        err_func,
        pglob
    );
    assert!(err_func.to_ptr().is_null()); // TODO

    // TODO: assert against passed flags
    assert!(flags & GLOB_NOSORT != 0);
    assert!(flags & GLOB_NOESCAPE != 0);
    let do_offs = flags & GLOB_DOOFFS != 0;

    // TODO: support other flags
    assert!(flags & !(GLOB_DOOFFS | GLOB_NOSORT | GLOB_NOESCAPE) == 0);

    let pattern_str = pattern_str.unwrap().to_owned();
    assert!(!pattern_str.contains('\\')); // TODO

    // TODO: account for non-global patterns
    assert!(pattern_str.starts_with("/"));
    let (directory, subpattern) = pattern_str.rsplit_once('/').unwrap();
    assert!(!directory.contains('*') && !directory.contains('?') && !directory.contains('[')); // TODO
    assert!(!subpattern.contains('?') && !subpattern.contains('[')); // TODO
    let has_star_wildcard = subpattern.contains('*');

    let directory_c_str = env.mem.alloc_and_write_cstr(directory.as_bytes());
    let dirp: MutPtr<DIR> = opendir(env, directory_c_str.cast_const());
    env.mem.free(directory_c_str.cast());
    assert!(!dirp.is_null());

    let subpattern_c_str: ConstPtr<u8> = env
        .mem
        .alloc_and_write_cstr(subpattern.as_bytes())
        .cast_const();

    let dirent_name_offset = std::mem::offset_of!(dirent, d_name) as GuestUSize;

    let mut next_dir_entry = readdir(env, dirp);
    let mut tmp_vec: Vec<MutPtr<u8>> = vec![];
    while !next_dir_entry.is_null() {
        let name_c_str: ConstPtr<u8> = next_dir_entry.cast().cast_const() + dirent_name_offset;

        // TODO: should we match on the whole path or just the filename?
        if fnmatch(env, subpattern_c_str, name_c_str, 0) == 0 {
            // TODO: use `lstat` and/or `stat` to get information on names found
            let name_len: GuestUSize = strlen(env, name_c_str);
            let dir_len = directory.len() as GuestUSize;
            let size = dir_len + 1 + name_len + 1;

            let buf = env.mem.calloc(size).cast::<u8>();
            env.mem
                .bytes_at_mut(buf, dir_len)
                .copy_from_slice(directory.as_bytes());
            env.mem.bytes_at_mut(buf + dir_len, 1).copy_from_slice(b"/");
            let name_bytes = env.mem.bytes_at(name_c_str, name_len).to_vec();
            env.mem
                .bytes_at_mut(buf + dir_len + 1, name_len)
                .copy_from_slice(&name_bytes);

            tmp_vec.push(buf);
        }

        next_dir_entry = readdir(env, dirp);
    }

    env.mem.free(subpattern_c_str.cast_mut().cast());
    closedir(env, dirp);

    let mut tmp_glob = env.mem.read(pglob);
    let offs = if do_offs { tmp_glob.gl_offs } else { 0 };
    let out_count = tmp_vec.len() as GuestUSize;
    let total_count = out_count + offs;
    tmp_glob.gl_pathc = out_count;
    tmp_glob.gl_matchc = out_count as i32;
    tmp_glob.gl_flags = if has_star_wildcard {
        flags | GLOB_MAGCHAR
    } else {
        flags & !GLOB_MAGCHAR
    };
    let list_out: MutPtr<MutPtr<u8>> = env
        .mem
        .calloc((total_count + 1) * guest_size_of::<MutPtr<u8>>())
        .cast();
    let start = list_out + offs;
    for (idx, out_str) in tmp_vec.iter().enumerate() {
        env.mem.write(start + idx as GuestUSize, *out_str);
    }
    tmp_glob.gl_pathv = list_out;
    env.mem.write(pglob, tmp_glob);

    if out_count > 0 {
        0 // success and match
    } else {
        GLOB_NOMATCH
    }
}

fn globfree(env: &mut Environment, pglob: MutPtr<glob_t>) {
    let tmp_glob = env.mem.read(pglob);
    let offs = if tmp_glob.gl_flags & GLOB_DOOFFS != 0 {
        tmp_glob.gl_offs
    } else {
        0
    };
    for i in 0..tmp_glob.gl_pathc as GuestUSize {
        let match_ = env.mem.read(tmp_glob.gl_pathv + offs + i);
        env.mem.free(match_.cast());
    }
    env.mem.free(tmp_glob.gl_pathv.cast());
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(glob(_, _, _, _)),
    export_c_func!(globfree(_)),
];
