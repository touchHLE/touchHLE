/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `dlfcn.h` (`dlopen()` and friends)
//! Реализация подсистемы динамического связывания POSIX для HLE-эмуляции.
//! Код спроектирован с учетом устойчивости к некорректному доступу к памяти со
//! стороны гостевого приложения.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutVoidPtr, Ptr};
use crate::Environment;

/// Псевдо-дескриптор для доступа к глобальной области видимости символов (main
//executable).
/// В операционных системах семейства Darwin/iOS RTLD_DEFAULT традиционно равен
//(void*)-2.
const RTLD_DEFAULT: MutVoidPtr = Ptr::from_bits(-2 as _);
const RTLD_NEXT: MutVoidPtr = Ptr::from_bits(-1 as _);
const RTLD_SELF: MutVoidPtr = Ptr::from_bits(-3 as _);
const RTLD_MAIN_ONLY: MutVoidPtr = Ptr::from_bits(-5 as _);

/// Checks whether the given handle is a special sentinel that should be
/// treated as a global symbol lookup scope.
fn is_global_scope_handle(handle: MutVoidPtr) -> bool {
    handle.is_null()
        || handle == RTLD_DEFAULT
        || handle == RTLD_NEXT
        || handle == RTLD_SELF
        || handle == RTLD_MAIN_ONLY
}

/// Проверяет, является ли запрашиваемая библиотека известной эмулятору
//(присутствует в статическом списке DYLIB_LIST).
fn is_known_library(path: &str) -> bool {
    crate::dyld::DYLIB_LIST
        .iter()
        .any(|dylib| dylib.path == path || dylib.aliases.contains(&path))
}

/// Реализация функции `dlopen` стандарта POSIX.
/// Загружает динамическую библиотеку в адресное пространство процесса (или
//симулирует этот процесс в HLE).
/// Возвращает дескриптор загруженной библиотеки или NULL в случае отсутствия
//файла или ошибки чтения.
fn dlopen(env: &mut Environment, path: ConstPtr<u8>, _mode: i32) -> MutVoidPtr {
    // В соответствии со стандартом POSIX, вызов dlopen(NULL) возвращает
    // дескриптор главной программы.
    // Эмулятор предоставляет доступ к глобальным символам через специальный
    // дескриптор RTLD_DEFAULT.
    if path.is_null() {
        return RTLD_DEFAULT;
    }

    // БЕЗОПАСНОСТЬ: Осуществляем защищенное чтение строки пути из
    // неконтролируемой гостевой памяти.
    // Если указатель недействителен (Out-Of-Bounds) или строка не является
    // корректной UTF-8 последовательностью,
    // мы прерываем операцию загрузки и возвращаем NULL, не допуская паники
    // эмулятора (Denial of Service).
    let path_str = match env.mem.cstr_at_utf8(path) {
        Ok(s) => s,
        Err(e) => {
            log!(
                "Warning: dlopen() failed to safely read path string from guest memory: {:?}",
                e
            );
            return Ptr::null();
        }
    };

    // Если библиотека не известна системе эмуляции (например,
    // кросс-платформенный фреймворк пытается
    // загрузить специфичный для другой платформы плагин), мы мягко отклоняем
    // запрос, возвращая NULL.
    // Данное поведение ожидается гостевым приложением для "мягкой деградации"
    // (graceful degradation).
    if !is_known_library(path_str) {
        log!(
            "Warning: dlopen() returning NULL for requested but unknown library: {}",
            path_str
        );
        return Ptr::null();
    }

    // Временная архитектура: использование указателя на строку пути в памяти
    // гостя как непрозрачного дескриптора.
    // TODO: Разработать защищенную систему управления дескрипторами (Handle
    // Allocator Table) на стороне хоста,
    // чтобы предотвратить уязвимости Use-After-Free, когда приложение
    // освобождает строку пути после вызова dlopen.
    path.cast_mut().cast()
}

/// Реализация функции `dlsym` стандарта POSIX.
/// Выполняет поиск адреса экспортированного символа (функции или переменной) в
//загруженном модуле.
fn dlsym(env: &mut Environment, handle: MutVoidPtr, symbol: ConstPtr<u8>) -> MutVoidPtr {
    // БЕЗОПАСНОСТЬ: Валидация переданного дескриптора.
    // Специальные дескрипторы (RTLD_DEFAULT, RTLD_NEXT, RTLD_SELF,
    // RTLD_MAIN_ONLY, а также NULL) означают глобальный поиск символа.
    if !is_global_scope_handle(handle) {
        let handle_path_ptr: ConstPtr<u8> = handle.cast().cast_const();
        let handle_str = match env.mem.cstr_at_utf8(handle_path_ptr) {
            Ok(s) => s,
            Err(_) => {
                log!("Warning: dlsym() returning NULL due to invalid or corrupted handle pointer (possible UAF)");
                return Ptr::null();
            }
        };

        // Если дескриптор указывает на строку, не являющуюся известной
        // библиотекой, запрос отклоняется.
        if !is_known_library(handle_str) {
            log!(
                "Warning: dlsym() returning NULL due to an unknown library handle: {}",
                handle_str
            );
            return Ptr::null();
        }
    }

    // БЕЗОПАСНОСТЬ: Защита от передачи NULL в качестве имени искомого символа.
    if symbol.is_null() {
        log!("Warning: dlsym() called with a NULL symbol pointer");
        return Ptr::null();
    }

    // БЕЗОПАСНОСТЬ: Чтение строкового имени символа из гостевой памяти.
    let symbol_str = match env.mem.cstr_at_utf8(symbol) {
        Ok(s) => s,
        Err(_) => {
            log!("Warning: dlsym() returning NULL due to invalid symbol string pointer in guest memory");
            return Ptr::null();
        }
    };

    // В бинарном формате Mach-O (платформы Apple) C-символы компилируются с
    // префиксом подчеркивания.
    let symbol_formatted = format!("_{}", symbol_str);

    // Попытка разрешить адрес через подсистему динамического загрузчика
    // эмулятора (dyld).
    // Функция create_proc_address безопасно вернет Err, если функция-заглушка
    // еще не реализована в эмуляторе.
    match env
        .dyld
        .create_proc_address(&mut env.mem, &mut env.cpu, &symbol_formatted)
    {
        Ok(addr) => Ptr::from_bits(addr.addr_with_thumb_bit()),
        Err(_) => {
            // POSIX `dlsym(3)`: returning NULL for an unknown symbol is the
            // documented success path for "this symbol does not exist".
            // In particular, Mono PInvoke handlers (`[DllImport("__Internal")]`
            // in C# code) probe dlsym to discover game-specific
            // entry points that live inside the guest binary and are
            // resolved separately by the Mono runtime. There's nothing
            // for the host to do for these symbols — emitting a Warning
            // every time turns a normal Mono cold start into hundreds
            // of log lines. Demoted to debug-only.
            log_dbg!(
                "dlsym({}): no host-side implementation, returning NULL",
                symbol_formatted
            );
            Ptr::null()
        }
    }
}

/// Реализация функции `dlclose` стандарта POSIX.
/// В HLE архитектуре выступает в роли заглушки, но строго соблюдает семантику
//возврата кодов ошибок.
fn dlclose(env: &mut Environment, handle: MutVoidPtr) -> i32 {
    if is_global_scope_handle(handle) {
        return 0; // Операция успешна
    }

    let handle_path_ptr: ConstPtr<u8> = handle.cast().cast_const();

    // БЕЗОПАСНОСТЬ: Проверяем валидность переданного дескриптора перед
    // возвратом кода статуса.
    match env.mem.cstr_at_utf8(handle_path_ptr) {
        Ok(handle_str) => {
            if !is_known_library(handle_str) {
                log!(
                    "Warning: dlclose() called on unknown or already freed library handle: {}",
                    handle_str
                );
                return -1; // -1 стандартный код ошибки POSIX для dlclose
            }
            0 // Успех
        }
        Err(_) => {
            log!("Warning: dlclose() called with an invalid or corrupted memory pointer");
            -1 // Ошибка доступа к памяти
        }
    }
}

/// Реализация функции `dlerror` стандарта POSIX (man 3 dlerror на Darwin).
///
/// Apple: "If no errors have occurred since initialization or since
/// `dlerror()` was last called, `dlerror()` returns NULL." Because our
/// `dlopen` / `dlsym` / `dlclose` never publish a per-thread error message
/// (they log internally and return NULL/-1 to the guest), the correct
/// POSIX-conforming reply is always `NULL`. Without this entry point the
/// dynamic linker installed a generic return-0 stub, which a few guests
/// (notably the iPhone OS port of `libstdc++`'s exception handling) treat
/// as "no error" but others (Bionic-style helpers) tried to call
/// `strlen()` on. Returning a real `NULL` pointer is unambiguous.
fn dlerror(_env: &mut Environment) -> ConstPtr<u8> {
    Ptr::null()
}

// Экспорт C-функций в глобальное адресное пространство гостевого процесса.
pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(dlopen(_, _)),
    export_c_func!(dlsym(_, _)),
    export_c_func!(dlclose(_)),
    export_c_func!(dlerror()),
];
