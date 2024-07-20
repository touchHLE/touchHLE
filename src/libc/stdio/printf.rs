/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `printf` function family. The implementation is also used by `NSLog` etc.

use crate::abi::{DotDotDot, VaList};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::{ns_string, unichar};
use crate::libc::clocale::{setlocale, LC_CTYPE};
use crate::libc::posix_io::{STDERR_FILENO, STDOUT_FILENO};
use crate::libc::stdio::FILE;
use crate::libc::stdlib::{atof_inner, atoi_inner, strtoul};
use crate::libc::string::strlen;
use crate::libc::wchar::wchar_t;
use crate::mem::{ConstPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{id, msg, nil};
use crate::Environment;
use std::collections::HashSet;
use std::io::Write;

const INTEGER_SPECIFIERS: [u8; 6] = [b'd', b'i', b'o', b'u', b'x', b'X'];
const FLOAT_SPECIFIERS: [u8; 3] = [b'f', b'e', b'g'];

/// String formatting implementation for `printf` and `NSLog` function families.
///
/// `NS_LOG` is [true] for the `NSLog` format string type, or [false] for the
/// `printf` format string type.
///
/// `get_format_char` is a callback that returns the byte at a given index in
/// the format string, or `'\0'` if the index is one past the last byte.
pub fn printf_inner<const NS_LOG: bool, F: Fn(&Mem, GuestUSize) -> u8>(
    env: &mut Environment,
    get_format_char: F,
    mut args: VaList,
) -> Vec<u8> {
    let mut res = Vec::<u8>::new();

    let mut format_char_idx = 0;

    loop {
        let c = get_format_char(&env.mem, format_char_idx);
        format_char_idx += 1;

        if c == b'\0' {
            break;
        }
        if c != b'%' {
            res.push(c);
            continue;
        }

        let pad_char = if get_format_char(&env.mem, format_char_idx) == b'0' {
            format_char_idx += 1;
            '0'
        } else {
            ' '
        };

        let pad_width = if get_format_char(&env.mem, format_char_idx) == b'*' {
            let pad_width = args.next::<i32>(env);
            assert!(pad_width >= 0); // TODO: Implement right-padding
            format_char_idx += 1;
            pad_width
        } else {
            let mut pad_width: i32 = 0;
            while let c @ b'0'..=b'9' = get_format_char(&env.mem, format_char_idx) {
                pad_width = pad_width * 10 + (c - b'0') as i32;
                format_char_idx += 1;
            }
            pad_width
        };

        let precision = if get_format_char(&env.mem, format_char_idx) == b'.' {
            format_char_idx += 1;
            let mut precision = 0;
            while let c @ b'0'..=b'9' = get_format_char(&env.mem, format_char_idx) {
                precision = precision * 10 + (c - b'0') as usize;
                format_char_idx += 1;
            }
            Some(precision)
        } else {
            None
        };

        let length_modifier = if get_format_char(&env.mem, format_char_idx) == b'l' {
            format_char_idx += 1;
            if get_format_char(&env.mem, format_char_idx) == b'l' {
                format_char_idx += 1;
                Some("ll")
            } else {
                Some("l")
            }
        } else {
            None
        };

        let specifier = get_format_char(&env.mem, format_char_idx);
        format_char_idx += 1;

        assert!(specifier != b'\0');
        if specifier == b'%' {
            res.push(b'%');
            continue;
        }

        if precision.is_some() {
            assert!(
                INTEGER_SPECIFIERS.contains(&specifier) || FLOAT_SPECIFIERS.contains(&specifier)
            )
        }

        match specifier {
            // Integer specifiers
            b'c' => {
                // TODO: support length modifier
                assert!(length_modifier.is_none());
                let c: u8 = args.next(env);
                assert!(pad_char == ' ' && pad_width == 0); // TODO
                res.push(c);
            }
            // Apple extension? Seemingly works in both NSLog and printf.
            b'C' => {
                assert!(length_modifier.is_none());
                let c: unichar = args.next(env);
                // TODO
                assert!(pad_char == ' ' && pad_width == 0);
                // This will panic if it's a surrogate! This isn't good if
                // targeting UTF-16 ([NSString stringWithFormat:] etc).
                let c = char::from_u32(c.into()).unwrap();
                write!(&mut res, "{}", c).unwrap();
            }
            b's' => {
                // TODO: support length modifier
                assert!(length_modifier.is_none());
                let c_string: ConstPtr<u8> = args.next(env);
                assert!(pad_char == ' ' && pad_width == 0); // TODO
                if !c_string.is_null() {
                    res.extend_from_slice(env.mem.cstr_at(c_string));
                } else {
                    res.extend_from_slice("(null)".as_bytes());
                }
            }
            b'S' => {
                // TODO: support length modifier
                assert!(length_modifier.is_none());
                // TODO: support other locales
                let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
                assert_eq!(env.mem.read(ctype_locale), b'C');
                let w_string: ConstPtr<wchar_t> = args.next(env);
                assert!(pad_char == ' ' && pad_width == 0); // TODO
                if !w_string.is_null() {
                    res.extend_from_slice(env.mem.wcstr_at(w_string).as_bytes());
                } else {
                    res.extend_from_slice("(null)".as_bytes());
                }
            }
            b'd' | b'i' | b'u' => {
                // Note: on 32-bit system int and long are i32,
                // so single length_modifier is ignored (but not double one!)
                let int: i64 = if specifier == b'u' {
                    if length_modifier == Some("ll") {
                        let uint: u64 = args.next(env);
                        uint.try_into().unwrap()
                    } else {
                        let uint: u32 = args.next(env);
                        uint.into()
                    }
                } else if length_modifier == Some("ll") {
                    args.next(env)
                } else {
                    let int: i32 = args.next(env);
                    int.into()
                };

                let int_with_precision = if precision.is_some_and(|value| value > 0) {
                    format!("{:01$}", int, precision.unwrap())
                } else {
                    format!("{}", int)
                };

                if pad_width > 0 {
                    let pad_width = pad_width as usize;
                    if pad_char == '0' && precision.is_none() {
                        write!(&mut res, "{:0>1$}", int_with_precision, pad_width).unwrap();
                    } else {
                        write!(&mut res, "{:>1$}", int_with_precision, pad_width).unwrap();
                    }
                } else {
                    res.extend_from_slice(int_with_precision.as_bytes());
                }
            }
            b'@' if NS_LOG => {
                assert!(length_modifier.is_none());
                let object: id = args.next(env);
                // TODO: use localized description if available?
                let description: id = msg![env; object description];
                if description != nil {
                    // TODO: avoid copy
                    // TODO: what if the description isn't valid UTF-16?
                    let description = ns_string::to_rust_string(env, description);
                    write!(&mut res, "{}", description).unwrap();
                } else {
                    write!(&mut res, "(null)").unwrap();
                }
            }
            b'x' => {
                assert!(precision.is_none());
                // Note: on 32-bit system unsigned int and unsigned long
                // are u32, so length_modifier is ignored
                let uint: u32 = args.next(env);
                if pad_width > 0 {
                    let pad_width = pad_width as usize;
                    if pad_char == '0' && precision.is_none() {
                        write!(&mut res, "{:0>1$x}", uint, pad_width).unwrap();
                    } else {
                        write!(&mut res, "{:>1$x}", uint, pad_width).unwrap();
                    }
                } else {
                    res.extend_from_slice(format!("{:x}", uint).as_bytes());
                }
            }
            b'X' => {
                assert!(precision.is_none());
                // Note: on 32-bit system unsigned int and unsigned long
                // are u32, so length_modifier is ignored
                let uint: u32 = args.next(env);
                if pad_width > 0 {
                    let pad_width = pad_width as usize;
                    if pad_char == '0' && precision.is_none() {
                        write!(&mut res, "{:0>1$X}", uint, pad_width).unwrap();
                    } else {
                        write!(&mut res, "{:>1$X}", uint, pad_width).unwrap();
                    }
                } else {
                    res.extend_from_slice(format!("{:X}", uint).as_bytes());
                }
            }
            b'p' => {
                assert!(length_modifier.is_none());
                let ptr: MutVoidPtr = args.next(env);
                res.extend_from_slice(format!("{:?}", ptr).as_bytes());
            }
            // Float specifiers
            b'f' => {
                let float: f64 = args.next(env);
                let pad_width = pad_width as usize;
                let precision = precision.unwrap_or(6);
                if pad_char == '0' {
                    res.extend_from_slice(
                        format!("{:01$.2$}", float, pad_width, precision).as_bytes(),
                    );
                } else {
                    res.extend_from_slice(
                        format!("{:1$.2$}", float, pad_width, precision).as_bytes(),
                    );
                }
            }
            b'e' => {
                let float: f64 = args.next(env);
                let pad_width = pad_width as usize;
                let precision = precision.unwrap_or(6);

                let exponent = float.abs().log10().floor();
                let mantissa = float.abs() / 10f64.powf(exponent);
                let sign = if float.is_sign_negative() { "-" } else { "" };
                if pad_char == '0' {
                    let float_exp_notation =
                        format!("{0:.1$}e{2:+03}", mantissa, precision, exponent);
                    res.extend_from_slice(
                        format!(
                            "{0}{1:0>2$}",
                            sign,
                            float_exp_notation,
                            pad_width.saturating_sub(sign.len())
                        )
                        .as_bytes(),
                    );
                } else {
                    let float_exp_notation =
                        format!("{0}{1:.2$}e{3:+03}", sign, mantissa, precision, exponent);
                    res.extend_from_slice(
                        format!("{0:>1$}", float_exp_notation, pad_width).as_bytes(),
                    );
                }
            }
            b'g' => {
                let float: f64 = args.next(env);
                let pad_width = pad_width as usize;

                let sign = if float.is_sign_negative() { "-" } else { "" };

                let formatted_f_without_padding_or_sign = {
                    // Precision in %g means max number of decimal digits in
                    // the mantissa. For that, we first calculate the length
                    // of the integer part and then we substract it from
                    // precision and use the result in the format! statement
                    let float_trunc_len = (float.abs().trunc() as i32).to_string().len();
                    // Format without padding
                    if let Some(precision) = precision {
                        format!(
                            "{:.1$}",
                            float.abs(),
                            precision.saturating_sub(float_trunc_len)
                        )
                    } else {
                        format!("{:.4}", float.abs())
                    }
                };
                let formatted_f = {
                    if pad_char == '0' {
                        format!(
                            "{}{:0>2$}",
                            sign,
                            formatted_f_without_padding_or_sign,
                            pad_width - sign.len()
                        )
                    } else {
                        let formatted_f_with_sign =
                            format!("{}{}", sign, formatted_f_without_padding_or_sign);
                        format!("{:>1$}", formatted_f_with_sign, pad_width)
                    }
                };

                let formatted_e_without_padding_or_sign = {
                    let exponent = float.abs().log10().floor();
                    let mantissa = float.abs() / 10f64.powf(exponent);
                    // Precision in %g means max number of decimal digits in
                    // the mantissa. For that, we first calculate the length
                    // of the mantissa's int part and then we substract it from
                    // precision and use the result in the format! statement
                    let mantissa_trunc_len = (mantissa.trunc() as i32).to_string().len();
                    // Format without padding
                    if let Some(precision) = precision {
                        if precision > mantissa_trunc_len {
                            format!(
                                "{0:.1$}e{2:+03}",
                                mantissa,
                                precision - mantissa_trunc_len,
                                exponent
                            )
                        } else {
                            format!("{:.0}e{:+03}", mantissa, exponent)
                        }
                    } else {
                        format!("{}e{:+03}", mantissa, exponent)
                    }
                };
                let formatted_e = if pad_char == '0' {
                    format!(
                        "{0}{1:0>2$}",
                        sign,
                        formatted_e_without_padding_or_sign,
                        pad_width.saturating_sub(sign.len())
                    )
                } else {
                    let without_padding_with_sign =
                        format!("{}{}", sign, formatted_e_without_padding_or_sign);
                    format!("{0:>1$}", without_padding_with_sign, pad_width)
                };

                // Use shortest formatted string
                let result = if formatted_e_without_padding_or_sign.len()
                    < formatted_f_without_padding_or_sign.len()
                    || precision.is_some_and(|x| x == 0)
                {
                    formatted_e
                } else {
                    formatted_f
                };

                res.extend_from_slice(result.as_bytes());
            }
            // TODO: more specifiers
            _ => unimplemented!(
                "Format character '{}'. Formatted up to index {}",
                specifier as char,
                format_char_idx
            ),
        }
    }

    log_dbg!("=> {:?}", std::str::from_utf8(&res));

    res
}

fn snprintf(
    env: &mut Environment,
    dest: MutPtr<u8>,
    n: GuestUSize,
    format: ConstPtr<u8>,
    args: DotDotDot,
) -> i32 {
    vsnprintf(env, dest, n, format, args.start())
}

fn vprintf(env: &mut Environment, format: ConstPtr<u8>, arg: VaList) -> i32 {
    log_dbg!(
        "vprintf({:?} ({:?}), ...)",
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), arg);
    // TODO: I/O error handling
    let _ = std::io::stdout().write_all(&res);
    res.len().try_into().unwrap()
}

fn vsnprintf(
    env: &mut Environment,
    dest: MutPtr<u8>,
    n: GuestUSize,
    format: ConstPtr<u8>,
    arg: VaList,
) -> i32 {
    log_dbg!(
        "vsnprintf({:?} {:?} {:?})",
        dest,
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), arg);
    let middle = if ((n - 1) as usize) < res.len() {
        &res[..(n - 1) as usize]
    } else {
        &res[..]
    };

    let dest_slice = env.mem.bytes_at_mut(dest, n);
    for (i, &byte) in middle.iter().chain(b"\0".iter()).enumerate() {
        dest_slice[i] = byte;
    }

    res.len().try_into().unwrap()
}

fn vsprintf(env: &mut Environment, dest: MutPtr<u8>, format: ConstPtr<u8>, arg: VaList) -> i32 {
    log_dbg!(
        "vsprintf({:?}, {:?} ({:?}), ...)",
        dest,
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), arg);

    let dest_slice = env
        .mem
        .bytes_at_mut(dest, (res.len() + 1).try_into().unwrap());
    for (i, &byte) in res.iter().chain(b"\0".iter()).enumerate() {
        dest_slice[i] = byte;
    }

    res.len().try_into().unwrap()
}

fn sprintf(env: &mut Environment, dest: MutPtr<u8>, format: ConstPtr<u8>, args: DotDotDot) -> i32 {
    log_dbg!(
        "sprintf({:?}, {:?} ({:?}), ...)",
        dest,
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), args.start());

    let dest_slice = env
        .mem
        .bytes_at_mut(dest, (res.len() + 1).try_into().unwrap());
    for (i, &byte) in res.iter().chain(b"\0".iter()).enumerate() {
        dest_slice[i] = byte;
    }

    res.len().try_into().unwrap()
}

fn swprintf(
    env: &mut Environment,
    ws: MutPtr<wchar_t>,
    n: GuestUSize,
    format: ConstPtr<wchar_t>,
    args: DotDotDot,
) -> i32 {
    // TODO: support other locales
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    let wcstr_format = env.mem.wcstr_at(format);
    log_dbg!(
        "swprintf({:?}, {}, {:?} ({:?}), ...)",
        ws,
        n,
        format,
        wcstr_format
    );

    let wcstr_format_bytes = wcstr_format.as_bytes();
    let len: GuestUSize = wcstr_format_bytes.len() as GuestUSize;
    let res = printf_inner::<false, _>(
        env,
        |_mem, idx| {
            if idx == len {
                b'\0'
            } else {
                wcstr_format_bytes[idx as usize]
            }
        },
        args.start(),
    );

    let to_write = n.min(res.len() as GuestUSize);
    for i in 0..to_write {
        env.mem.write(ws + i, res[i as usize] as wchar_t);
    }
    if to_write >= n {
        // TODO: set errno
        return -1;
    }
    env.mem.write(ws + to_write, wchar_t::default());
    to_write as i32
}

fn printf(env: &mut Environment, format: ConstPtr<u8>, args: DotDotDot) -> i32 {
    log_dbg!(
        "printf({:?} ({:?}), ...)",
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), args.start());
    // TODO: I/O error handling
    let _ = std::io::stdout().write_all(&res);
    res.len().try_into().unwrap()
}

// TODO: more printf variants

fn sscanf_common(
    env: &mut Environment,
    src: ConstPtr<u8>,
    format: ConstPtr<u8>,
    mut args: VaList,
) -> i32 {
    let mut src_ptr = src.cast_mut();
    let mut format_char_idx = 0;

    let mut matched_args = 0;

    loop {
        let c = env.mem.read(format + format_char_idx);
        format_char_idx += 1;

        if c == b'\0' {
            break;
        }
        if c != b'%' {
            if isspace(env, format + format_char_idx - 1) && isspace(env, src_ptr.cast_const()) {
                while isspace(env, src_ptr.cast_const()) {
                    src_ptr += 1;
                }
                continue;
            }
            let cc = env.mem.read(src_ptr);
            if c != cc {
                return matched_args;
            }
            src_ptr += 1;
            continue;
        }

        let mut max_width: i32 = 0;
        while let c @ b'0'..=b'9' = env.mem.read(format + format_char_idx) {
            max_width = max_width * 10 + (c - b'0') as i32;
            format_char_idx += 1;
        }

        let length_modifier = if env.mem.read(format + format_char_idx) == b'h' {
            format_char_idx += 1;
            Some(b'h')
        } else {
            None
        };

        let specifier = env.mem.read(format + format_char_idx);
        format_char_idx += 1;

        match specifier {
            b'd' | b'i' => {
                if specifier == b'i' {
                    // TODO: hexs and octals
                    assert_ne!(env.mem.read(src_ptr), b'0');
                }

                match length_modifier {
                    Some(lm) => {
                        match lm {
                            b'h' => {
                                // signed short* or unsigned short*
                                match atoi_inner(env, src_ptr.cast_const()) {
                                    Ok((val, len)) => {
                                        if max_width > 0 {
                                            assert_eq!(max_width, len as i32);
                                        }
                                        src_ptr += len;
                                        let c_int_ptr: ConstPtr<i16> = args.next(env);
                                        env.mem
                                            .write(c_int_ptr.cast_mut(), val.try_into().unwrap());
                                    }
                                    Err(_) => break,
                                }
                            }
                            _ => unimplemented!(),
                        }
                    }
                    _ => match atoi_inner(env, src_ptr.cast_const()) {
                        Ok((val, len)) => {
                            src_ptr += len;
                            let c_int_ptr: ConstPtr<i32> = args.next(env);
                            env.mem.write(c_int_ptr.cast_mut(), val);
                        }
                        Err(_) => break,
                    },
                }
            }
            b'f' => {
                assert_eq!(max_width, 0);
                assert!(length_modifier.is_none());
                match atof_inner(env, src_ptr.cast_const()) {
                    Ok((val, len)) => {
                        src_ptr += len;
                        let c_int_ptr: ConstPtr<f32> = args.next(env);
                        env.mem.write(c_int_ptr.cast_mut(), val as f32);
                    }
                    Err(_) => break,
                }
            }
            b'x' | b'X' => {
                let c_len: GuestUSize = strlen(env, src_ptr.cast_const());
                if max_width != 0 {
                    assert_eq!(c_len, max_width.try_into().unwrap());
                }
                let val: u32 = strtoul(env, src_ptr.cast_const(), Ptr::null(), 16);
                src_ptr += c_len;
                let c_u32_ptr: ConstPtr<u32> = args.next(env);
                env.mem.write(c_u32_ptr.cast_mut(), val);
            }
            b'[' => {
                assert_eq!(max_width, 0);
                assert!(length_modifier.is_none());
                // TODO: support ranges like [0-9]
                // [set] case
                let mut c = env.mem.read(format + format_char_idx);
                format_char_idx += 1;
                // TODO: only `not in the set` for a moment
                assert_eq!(c, b'^');
                // Build set
                let mut set: HashSet<u8> = HashSet::new();
                // TODO: set can contain ']' as well
                c = env.mem.read(format + format_char_idx);
                format_char_idx += 1;
                while c != b']' {
                    set.insert(c);
                    c = env.mem.read(format + format_char_idx);
                    format_char_idx += 1;
                }
                let mut dst_ptr: MutPtr<u8> = args.next(env);
                // Consume `src` while chars are not in the set
                let mut cc = env.mem.read(src_ptr);
                src_ptr += 1;
                // TODO: handle end of src string
                while !set.contains(&cc) {
                    env.mem.write(dst_ptr, cc);
                    dst_ptr += 1;
                    cc = env.mem.read(src_ptr);
                    src_ptr += 1;
                }
                // we need to backtrack one position
                src_ptr -= 1;
                env.mem.write(dst_ptr, b'\0');
            }
            b's' => {
                assert_eq!(max_width, 0);
                assert!(length_modifier.is_none());
                let mut dst_ptr: MutPtr<u8> = args.next(env);
                loop {
                    if !isspace(env, src_ptr.cast_const()) {
                        env.mem.write(dst_ptr, env.mem.read(src_ptr));
                        src_ptr += 1;
                        dst_ptr += 1;
                    } else {
                        break;
                    }
                }
                env.mem.write(dst_ptr, b'\0');
            }
            // TODO: more specifiers
            _ => unimplemented!("Format character '{}'", specifier as char),
        }

        matched_args += 1;
    }

    matched_args
}

fn sscanf(env: &mut Environment, src: ConstPtr<u8>, format: ConstPtr<u8>, args: DotDotDot) -> i32 {
    log_dbg!(
        "sscanf({:?} ({:?}), {:?} ({:?}), ...)",
        src,
        env.mem.cstr_at_utf8(src),
        format,
        env.mem.cstr_at_utf8(format)
    );

    sscanf_common(env, src, format, args.start())
}

fn swscanf(
    env: &mut Environment,
    ws: ConstPtr<wchar_t>,
    format: ConstPtr<wchar_t>,
    args: DotDotDot,
) -> i32 {
    // TODO: support other locales
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    let w_string = env.mem.wcstr_at(ws);
    let w_format = env.mem.wcstr_at(format);
    log_dbg!(
        "swscanf({:?} ({:?}), {:?} ({:?}), ...)",
        ws,
        w_string,
        format,
        w_format
    );
    // TODO: refactor code to parametrise sscanf_common()
    // for normal and wide strings instead
    let c_string = env.mem.alloc_and_write_cstr(w_string.as_bytes());
    let c_format = env.mem.alloc_and_write_cstr(w_format.as_bytes());
    let res = sscanf(env, c_string.cast_const(), c_format.cast_const(), args);
    env.mem.free(c_string.cast());
    env.mem.free(c_format.cast());
    res
}

fn vsscanf(env: &mut Environment, src: ConstPtr<u8>, format: ConstPtr<u8>, arg: VaList) -> i32 {
    log_dbg!(
        "vsscanf({:?}, {:?} ({:?}), ...)",
        src,
        format,
        env.mem.cstr_at_utf8(format)
    );

    sscanf_common(env, src, format, arg)
}

fn fprintf(
    env: &mut Environment,
    stream: MutPtr<FILE>,
    format: ConstPtr<u8>,
    args: DotDotDot,
) -> i32 {
    log_dbg!(
        "fprintf({:?}, {:?} ({:?}), ...)",
        stream,
        format,
        env.mem.cstr_at_utf8(format)
    );

    vfprintf(env, stream, format, args.start())
}

fn vfprintf(env: &mut Environment, stream: MutPtr<FILE>, format: ConstPtr<u8>, arg: VaList) -> i32 {
    log_dbg!(
        "vfprintf({:?}, {:?} ({:?}), ...)",
        stream,
        format,
        env.mem.cstr_at_utf8(format)
    );

    let res = printf_inner::<false, _>(env, |mem, idx| mem.read(format + idx), arg);
    // TODO: I/O error handling
    match env.mem.read(stream).fd {
        STDOUT_FILENO => _ = std::io::stdout().write_all(&res),
        STDERR_FILENO => _ = std::io::stderr().write_all(&res),
        _ => unimplemented!(),
    }
    res.len().try_into().unwrap()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sscanf(_, _, _)),
    export_c_func!(swscanf(_, _, _)),
    export_c_func!(vsscanf(_, _, _)),
    export_c_func!(snprintf(_, _, _, _)),
    export_c_func!(vprintf(_, _)),
    export_c_func!(vsnprintf(_, _, _, _)),
    export_c_func!(vsprintf(_, _, _)),
    export_c_func!(sprintf(_, _, _)),
    export_c_func!(swprintf(_, _, _, _)),
    export_c_func!(printf(_, _)),
    export_c_func!(fprintf(_, _, _)),
    export_c_func!(vfprintf(_, _, _)),
];

// Helper function, not a part of printf family
// TODO: write proper libc's isspace()
pub fn isspace(env: &mut Environment, src: ConstPtr<u8>) -> bool {
    let c = env.mem.read(src);
    // Rust's definition of whitespace excludes vertical tab, unlike C's
    c.is_ascii_whitespace() || c == b'\x0b'
}
