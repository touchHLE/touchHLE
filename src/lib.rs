/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[macro_use]
pub mod log;

pub mod abi;
pub mod cpu;
pub mod dyld;
pub mod fs;
pub mod libc;
pub mod mem;

use crate::cpu::Cpu;
use crate::fs::FileSystem;
use crate::mem::Memory;

/// Главная структура окружения эмулятора
pub struct Environment {
    pub cpu: Box<dyn Cpu>,
    pub mem: Memory,
    pub fs: FileSystem,
    pub libc_state: LibcState,
}

impl Environment {
    pub fn new(cpu: Box<dyn Cpu>, mem: Memory, fs: FileSystem) -> Self {
        Self {
            cpu,
            mem,
            fs,
            libc_state: LibcState::default(),
        }
    }
}

/// Состояние стандартной библиотеки C и системных вызовов
#[derive(Default)]
pub struct LibcState {
    pub posix_io: libc::posix_io::State,
    pub errno: i32,
    // Сюда можно добавить состояние для malloc, сетей и т.д.
}

pub fn step(env: &mut Environment) {
    env.cpu.step(&mut env.mem);
}

// Вспомогательные макросы для логирования (если они не вынесены в log.rs)
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[INFO] {}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_dbg {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!("[DEBUG] {}", format_args!($($arg)*));
    };
}

