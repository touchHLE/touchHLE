/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! touchHLE is a high-level emulator (HLE) for iPhone OS applications.

// Allow the crate to have a non-snake-case name (touchHLE).
#![allow(non_snake_case)]
#![allow(rustdoc::private_intra_doc_links)]

#[macro_use]
mod log;
mod abi;
mod audio;
mod bundle;
mod cpu;
mod debug;
mod dyld;
mod environment;
mod font;
mod frameworks;
mod fs;
mod gdb;
mod gles;
mod image;

// Секция модулей libc
pub mod libc {
    pub mod stdio;
    pub mod stdlib;
    pub mod string;
    pub mod sqlite; // ДОБАВЛЕНО: Регистрация нашего нового модуля SQLite
    pub mod clocale;
    pub mod ctype;
    pub mod dirent;
    pub mod errno;
    pub mod fcntl;
    pub mod iconv;
    pub mod locale;
    pub mod math;
    pub mod mman;
    pub mod posix_io;
    pub mod pthread;
    pub mod pwd;
    pub mod resource;
    pub mod setjmp;
    pub mod signal;
    pub mod stat;
    pub mod stdarg;
    pub mod stdint;
    pub mod sys_ctl;
    pub mod sys_time;
    pub mod termios;
    pub mod time;
    pub mod unistd;
    pub mod utime;
    pub mod wchar;
}

mod licenses;
mod mach_o;
mod matrix;
mod mem;
mod objc;
mod options;
mod paths;
mod stack;
mod window;

use environment::{Environment, MutexId, MutexType, ThreadId, PTHREAD_MUTEX_DEFAULT};
use std::path::PathBuf;

pub use touchHLE_version::*;

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: std::ffi::c_int,
    _argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s
        } else {
            "(non-string payload)"
        };
        if let Some(location) = info.location() {
            echo!("Panic at {}: {}", location, payload);
        } else {
            echo!("Panic: {}", payload);
        }
    }));

    match main([String::new()].into_iter()) {
        Ok(_) => echo!("touchHLE finished"),
        Err(e) => echo!("touchHLE errored: {e:?}"),
    }
    0
}

const USAGE: &str = "\
Usage:
    touchHLE [PATH] [OPTIONS]
... (сокращено для краткости, остальной текст USAGE без изменений)
";

pub fn main<T: Iterator<Item = String>>(mut args: T) -> Result<(), String> {
    echo!(
        "touchHLE {}{}{} — https://touchhle.org/",
        branding(),
        if branding().is_empty() { "" } else { " " },
        VERSION,
    );
    
    // ... (весь остальной код функции main остается без изменений, 
    // так как логика инициализации модулей происходит в Environment::new)
    
    // Код main из твоего предыдущего сообщения полностью совместим.
    // Единственное важное изменение здесь — структура `pub mod libc`.
    
    // (Я опускаю повторение 200+ строк неизмененного кода main, 
    // чтобы не превышать лимит сообщения, просто убедись, 
    // что блок pub mod libc выглядит как выше).
    Ok(())
}

