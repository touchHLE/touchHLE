/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stack Smashing Protection (SSP)

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::environment::Environment;

// Если защита стека поймает переполнение (буфер оверфлоу), игра вызовет эту
// функцию.
// На реальном iOS этот вызов аборт-ит гостевой процесс, а не хост. Чтобы не
// ронять весь эмулятор из-за бага в одной игре, логируем громко и
// возвращаемся: пусть гость продолжит работу до следующей фатальной ошибки
// (которая, если что, тоже будет защищена аналогичной обработкой).
pub fn __stack_chk_fail(_env: &mut Environment) {
    log!(
        "*** __stack_chk_fail: stack smashing detected in guest! The guest's stack canary was \
         corrupted. This usually means the app has a real buffer overflow bug. On real iOS this \
         would abort the process; the emulator will keep running but the app may behave \
         unpredictably from this point on."
    );
}

pub const FUNCTIONS: FunctionExports = &[
    // Экспортируем функцию. Макрос автоматически добавит нужное подчеркивание
    // для C.
    export_c_func!(__stack_chk_fail()),
];

pub const CONSTANTS: ConstantExports = &[
    // Используем гарантированно существующий вариант.
    // Игра получит валидный указатель на 0x00000000 и использует его как
    // канарейку.
    ("___stack_chk_guard", HostConstant::NullPtr),
];
