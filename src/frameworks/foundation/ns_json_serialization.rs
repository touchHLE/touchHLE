/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSJSONSerialization` — convert between an Objective-C property-list-like
//! object graph and JSON encoded as `NSData`.
//!
//! Implements Apple's documented public API:
//! <https://developer.apple.com/documentation/foundation/nsjsonserialization>
//!
//! The supported object types match Apple's:
//!
//! * `NSDictionary` (keys must be `NSString`s) ↔ JSON object
//! * `NSArray`                              ↔ JSON array
//! * `NSString`                             ↔ JSON string
//! * `NSNumber`                             ↔ JSON number/`true`/`false`
//! * `NSNull`                               ↔ JSON `null`
//!
//! Anything else is rejected by `+isValidJSONObject:` and causes
//! `+dataWithJSONObject:options:error:` to return `nil`, matching the
//! Foundation reference implementation.

use super::ns_array::ArrayHostObject;
use super::ns_dictionary::DictionaryHostObject;
use super::ns_string::{from_rust_string, to_rust_string};
use super::ns_value::NSNumberHostObject;
use super::NSUInteger;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, ClassExports,
};
use crate::Environment;

pub type NSJSONReadingOptions = NSUInteger;
pub const NSJSONReadingMutableContainers: NSJSONReadingOptions = 1 << 0;
pub const NSJSONReadingMutableLeaves: NSJSONReadingOptions = 1 << 1;
pub const NSJSONReadingAllowFragments: NSJSONReadingOptions = 1 << 2;

pub type NSJSONWritingOptions = NSUInteger;
pub const NSJSONWritingPrettyPrinted: NSJSONWritingOptions = 1 << 0;

pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

@implementation NSJSONSerialization: NSObject

+ (bool)isValidJSONObject:(id)obj {
    is_valid_json_object_root(env, obj)
}

+ (id)dataWithJSONObject:(id)obj
                 options:(NSJSONWritingOptions)opt
                   error:(MutPtr<id>)error { // NSError **
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    if !is_valid_json_object_root(env, obj) {
        log!("NSJSONSerialization: top-level object is not a valid JSON object");
        return nil;
    }
    let pretty = (opt & NSJSONWritingPrettyPrinted) != 0;
    let mut out = String::new();
    encode_value(env, obj, &mut out, pretty, 0);
    let bytes = out.into_bytes();
    let len: u32 = bytes.len().try_into().unwrap();
    let ptr = env.mem.alloc(len.max(1));
    if len > 0 {
        env.mem.bytes_at_mut(ptr.cast(), len).copy_from_slice(&bytes);
    }
    msg_class![env; NSData dataWithBytesNoCopy:ptr length:len]
}

+ (id)JSONObjectWithData:(id)data // NSData *
                 options:(NSJSONReadingOptions)opt
                   error:(MutPtr<id>)error { // NSError **
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    if data == nil {
        return nil;
    }
    let slice = crate::frameworks::foundation::ns_data::to_rust_slice(env, data).to_vec();
    let text = match std::str::from_utf8(&slice) {
        Ok(s) => s.to_string(),
        Err(_) => {
            log!("NSJSONSerialization: input data is not valid UTF-8");
            return nil;
        }
    };
    let mut parser = JsonParser::new(&text);
    parser.skip_ws();
    let result = match parser.parse_value(env, opt) {
        Ok(v) => v,
        Err(e) => {
            log!("NSJSONSerialization: parse error: {}", e);
            return nil;
        }
    };
    parser.skip_ws();
    if !parser.eof() {
        log!("NSJSONSerialization: trailing characters after JSON root");
        return nil;
    }
    let allow_fragments = (opt & NSJSONReadingAllowFragments) != 0;
    if !allow_fragments {
        let arr_class = env.objc.get_known_class("NSArray", &mut env.mem);
        let dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
        let is_container: bool = msg![env; result isKindOfClass:arr_class]
            || msg![env; result isKindOfClass:dict_class];
        if !is_container {
            log!("NSJSONSerialization: top-level value is not an array or dictionary");
            return nil;
        }
    }
    autorelease(env, result)
}

@end

};

fn is_valid_json_object_root(env: &mut Environment, obj: id) -> bool {
    if obj == nil {
        return false;
    }
    let ns_array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    let ns_dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    let is_array: bool = msg![env; obj isKindOfClass:ns_array_class];
    let is_dict: bool = msg![env; obj isKindOfClass:ns_dict_class];
    if !is_array && !is_dict {
        return false;
    }
    is_valid_json_subtree(env, obj)
}

fn is_valid_json_subtree(env: &mut Environment, obj: id) -> bool {
    if obj == nil {
        return false;
    }
    let ns_array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    let ns_dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    let ns_number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    let ns_null_class = env.objc.get_known_class("NSNull", &mut env.mem);

    if msg![env; obj isKindOfClass:ns_string_class]
        || msg![env; obj isKindOfClass:ns_number_class]
        || msg![env; obj isKindOfClass:ns_null_class]
    {
        return true;
    }
    if msg![env; obj isKindOfClass:ns_array_class] {
        let array = env.objc.borrow::<ArrayHostObject>(obj).array.clone();
        for item in array {
            if !is_valid_json_subtree(env, item) {
                return false;
            }
        }
        return true;
    }
    if msg![env; obj isKindOfClass:ns_dict_class] {
        let pairs = collect_dictionary_pairs(env, obj);
        for (k, v) in pairs {
            if !msg![env; k isKindOfClass:ns_string_class] {
                return false;
            }
            if !is_valid_json_subtree(env, v) {
                return false;
            }
        }
        return true;
    }
    false
}

fn collect_dictionary_pairs(env: &mut Environment, dict: id) -> Vec<(id, id)> {
    let host: &DictionaryHostObject = env.objc.borrow(dict);
    let mut out = Vec::new();
    for collisions in host.map.values() {
        for &(k, v) in collisions {
            out.push((k, v));
        }
    }
    out
}

fn encode_value(env: &mut Environment, obj: id, out: &mut String, pretty: bool, depth: usize) {
    if obj == nil {
        out.push_str("null");
        return;
    }
    let ns_null_class = env.objc.get_known_class("NSNull", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_null_class] {
        out.push_str("null");
        return;
    }
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_string_class] {
        let s = to_rust_string(env, obj).into_owned();
        encode_string(&s, out);
        return;
    }
    let ns_number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_number_class] {
        encode_number(env, obj, out);
        return;
    }
    let ns_array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_array_class] {
        let items = env.objc.borrow::<ArrayHostObject>(obj).array.clone();
        if items.is_empty() {
            out.push_str("[]");
            return;
        }
        out.push('[');
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if pretty {
                out.push('\n');
                push_indent(out, depth + 1);
            }
            encode_value(env, *item, out, pretty, depth + 1);
        }
        if pretty {
            out.push('\n');
            push_indent(out, depth);
        }
        out.push(']');
        return;
    }
    let ns_dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_dict_class] {
        let pairs = collect_dictionary_pairs(env, obj);
        if pairs.is_empty() {
            out.push_str("{}");
            return;
        }
        out.push('{');
        for (i, (k, v)) in pairs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if pretty {
                out.push('\n');
                push_indent(out, depth + 1);
            }
            let ks = to_rust_string(env, *k).into_owned();
            encode_string(&ks, out);
            out.push(':');
            if pretty {
                out.push(' ');
            }
            encode_value(env, *v, out, pretty, depth + 1);
        }
        if pretty {
            out.push('\n');
            push_indent(out, depth);
        }
        out.push('}');
        return;
    }
    out.push_str("null");
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn encode_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn encode_number(env: &mut Environment, num: id, out: &mut String) {
    // NSNumber wraps a boolean if its objCType is "c" (signed char) or "B"
    // (bool) and the value is 0 or 1. Apple's JSON encoder writes such
    // values as `true`/`false`. We approximate this by checking the host
    // object's type tag.
    let host = env.objc.borrow::<NSNumberHostObject>(num);
    match host {
        NSNumberHostObject::Bool(b) => {
            out.push_str(if *b { "true" } else { "false" });
        }
        NSNumberHostObject::Int(i) => {
            out.push_str(&i.to_string());
        }
        NSNumberHostObject::UnsignedInt(u) => {
            out.push_str(&u.to_string());
        }
        NSNumberHostObject::LongLong(i) => {
            out.push_str(&i.to_string());
        }
        NSNumberHostObject::UnsignedLongLong(u) => {
            out.push_str(&u.to_string());
        }
        NSNumberHostObject::Float(f) => {
            let f = *f as f64;
            if f.is_finite() {
                out.push_str(&format_finite_f64(f));
            } else {
                // JSON has no representation for NaN/Inf. Apple's
                // implementation throws/returns nil from
                // dataWithJSONObject:; we substitute "null" defensively.
                out.push_str("null");
            }
        }
        NSNumberHostObject::Double(f) => {
            if f.is_finite() {
                out.push_str(&format_finite_f64(*f));
            } else {
                out.push_str("null");
            }
        }
        NSNumberHostObject::Short(i) => {
            out.push_str(&i.to_string());
        }
        NSNumberHostObject::UnsignedShort(u) => {
            out.push_str(&u.to_string());
        }
        NSNumberHostObject::Char(i) => {
            out.push_str(&i.to_string());
        }
    }
}

fn format_finite_f64(f: f64) -> String {
    if f == f.trunc() && f.abs() < 1e16 {
        format!("{}", f as i64)
    } else {
        let s = format!("{}", f);
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{}.0", s)
        }
    }
}

// ---------------------------------------------------------------------------
// Hand-rolled JSON parser. Matches the subset Apple's implementation
// accepts (RFC 8259 — no comments, no trailing commas).
// ---------------------------------------------------------------------------

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        JsonParser {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, lit: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.pos..self.pos + lit.len()) == Some(lit) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(format!("expected literal {:?} at offset {}", std::str::from_utf8(lit).unwrap_or("?"), self.pos))
        }
    }

    fn parse_value(&mut self, env: &mut Environment, opt: NSJSONReadingOptions) -> Result<id, String> {
        self.skip_ws();
        let c = self.peek().ok_or_else(|| "unexpected EOF".to_string())?;
        match c {
            b'{' => self.parse_object(env, opt),
            b'[' => self.parse_array(env, opt),
            b'"' => {
                let s = self.parse_string()?;
                Ok(from_rust_string(env, s))
            }
            b't' => {
                self.expect(b"true")?;
                Ok(msg_class![env; NSNumber numberWithBool:true])
            }
            b'f' => {
                self.expect(b"false")?;
                Ok(msg_class![env; NSNumber numberWithBool:false])
            }
            b'n' => {
                self.expect(b"null")?;
                Ok(msg_class![env; NSNull null])
            }
            b'-' | b'0'..=b'9' => self.parse_number(env),
            other => Err(format!("unexpected byte 0x{:02x} at offset {}", other, self.pos)),
        }
    }

    fn parse_object(&mut self, env: &mut Environment, opt: NSJSONReadingOptions) -> Result<id, String> {
        self.bump(); // '{'
        let mutable = (opt & NSJSONReadingMutableContainers) != 0;
        let dict: id = if mutable {
            msg_class![env; NSMutableDictionary dictionary]
        } else {
            msg_class![env; NSMutableDictionary dictionary]
        };
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            if mutable {
                return Ok(dict);
            }
            let frozen: id = msg![env; dict copy];
            return Ok(crate::objc::autorelease(env, frozen));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(format!("expected string key at offset {}", self.pos));
            }
            let key_str = self.parse_string()?;
            let key_obj = from_rust_string(env, key_str);
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(format!("expected ':' at offset {}", self.pos));
            }
            let val = self.parse_value(env, opt)?;
            let _: () = msg![env; dict setObject:val forKey:key_obj];
            crate::objc::release(env, key_obj);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                    continue;
                }
                Some(b'}') => {
                    self.bump();
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at offset {}", self.pos)),
            }
        }
        if mutable {
            Ok(dict)
        } else {
            let frozen: id = msg![env; dict copy];
            Ok(crate::objc::autorelease(env, frozen))
        }
    }

    fn parse_array(&mut self, env: &mut Environment, opt: NSJSONReadingOptions) -> Result<id, String> {
        self.bump(); // '['
        let mutable = (opt & NSJSONReadingMutableContainers) != 0;
        let arr: id = msg_class![env; NSMutableArray array];
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            if mutable {
                return Ok(arr);
            }
            let frozen: id = msg![env; arr copy];
            return Ok(crate::objc::autorelease(env, frozen));
        }
        loop {
            let val = self.parse_value(env, opt)?;
            let _: () = msg![env; arr addObject:val];
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                    continue;
                }
                Some(b']') => {
                    self.bump();
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at offset {}", self.pos)),
            }
        }
        if mutable {
            Ok(arr)
        } else {
            let frozen: id = msg![env; arr copy];
            Ok(crate::objc::autorelease(env, frozen))
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.bump() != Some(b'"') {
            return Err(format!("expected '\"' at offset {}", self.pos.saturating_sub(1)));
        }
        let mut out = String::new();
        loop {
            let b = self
                .bump()
                .ok_or_else(|| "unterminated string".to_string())?;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self.bump().ok_or_else(|| "bad escape".to_string())?;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\x08'),
                        b'f' => out.push('\x0c'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&code) {
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return Err("expected low surrogate".to_string());
                                }
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err("invalid low surrogate".to_string());
                                }
                                let cp = 0x10000
                                    + (((code - 0xD800) as u32) << 10)
                                    + ((low - 0xDC00) as u32);
                                if let Some(c) = char::from_u32(cp) {
                                    out.push(c);
                                } else {
                                    return Err("invalid surrogate pair".to_string());
                                }
                            } else if let Some(c) = char::from_u32(code as u32) {
                                out.push(c);
                            } else {
                                return Err("invalid \\u escape".to_string());
                            }
                        }
                        other => return Err(format!("bad escape \\{}", other as char)),
                    }
                }
                b if b < 0x20 => return Err("unescaped control char in string".to_string()),
                b => {
                    // Continue collecting UTF-8 bytes until valid codepoint.
                    out.push(b as char);
                    // The above only works for ASCII; for multi-byte UTF-8
                    // we need to keep gathering. Rewind one and grab the
                    // whole codepoint via str slicing.
                    if b >= 0x80 {
                        out.pop();
                        let start = self.pos - 1;
                        let s = std::str::from_utf8(&self.bytes[start..])
                            .map_err(|_| "invalid UTF-8 in string".to_string())?;
                        let ch = s.chars().next().ok_or_else(|| "invalid UTF-8".to_string())?;
                        out.push(ch);
                        self.pos = start + ch.len_utf8();
                    }
                }
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u16, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let b = self.bump().ok_or_else(|| "short \\u escape".to_string())?;
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err("bad hex digit".to_string()),
            };
            v = (v << 4) | d as u32;
        }
        Ok(v as u16)
    }

    fn parse_number(&mut self, env: &mut Environment) -> Result<id, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some(b'.') {
            is_float = true;
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.bump();
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.bump();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        let slice = &self.bytes[start..self.pos];
        let text = std::str::from_utf8(slice).map_err(|_| "bad number".to_string())?;
        if is_float {
            let f: f64 = text.parse().map_err(|_| format!("bad number {:?}", text))?;
            Ok(msg_class![env; NSNumber numberWithDouble:f])
        } else if let Ok(i) = text.parse::<i64>() {
            Ok(msg_class![env; NSNumber numberWithLongLong:i])
        } else if let Ok(u) = text.parse::<u64>() {
            Ok(msg_class![env; NSNumber numberWithUnsignedLongLong:u])
        } else {
            let f: f64 = text.parse().map_err(|_| format!("bad number {:?}", text))?;
            Ok(msg_class![env; NSNumber numberWithDouble:f])
        }
    }
}
