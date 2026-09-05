/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGFont`

use super::cg_data_provider;
use super::cg_data_provider::CGDataProviderRef;
use super::{CGFloat, CGPoint, CGRect, CGSize};
use crate::dyld::{export_c_func, FunctionExports};
use crate::font::Font;
use crate::frameworks::core_foundation::cf_data::CFDataRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_geometry::CGRectZero;
use crate::frameworks::foundation::unichar;
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject};
use crate::Environment;

// Note: on iOS SDK side this type is defined as a pointer to an opaque struct
pub(super) type CGFontRef = CFTypeRef;

type CGFontIndex = u16;
pub(super) type CGGlyph = CGFontIndex;

pub struct CGFontHostObject {
    pub font: Font,
}
impl HostObject for CGFontHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGFont: NSObject
@end

};

fn CGFontCreateWithDataProvider(env: &mut Environment, provider: CGDataProviderRef) -> CGFontRef {
    let bytes = cg_data_provider::borrow_bytes(env, provider);
    let font = Font::from_vec(bytes.to_vec());
    let host_obj = Box::new(CGFontHostObject { font });
    let class = env.objc.get_known_class("_touchHLE_CGFont", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

pub fn CGFontRetain(env: &mut Environment, font: CGFontRef) -> CGFontRef {
    if !font.is_null() {
        CFRetain(env, font)
    } else {
        font
    }
}
pub fn CGFontRelease(env: &mut Environment, font: CGFontRef) {
    if !font.is_null() {
        CFRelease(env, font);
    }
}

// This is an undocumented API! But some apps still may call it
fn CGFontGetGlyphsForUnichars(
    env: &mut Environment,
    font: CGFontRef,
    chars: ConstPtr<unichar>,
    glyphs: MutPtr<CGGlyph>,
    length: GuestUSize,
) -> bool {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    for i in 0..length {
        let c: unichar = env.mem.read(chars + i);
        let x = font.glyph_id_for_char(c).0;
        env.mem.write(glyphs + i, x);
    }
    true
}

fn CGFontGetUnitsPerEm(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.units_per_em().into()
}

fn CGFontGetAscent(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.ascent_unscaled() as i32
}
fn CGFontGetDescent(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.descent_unscaled() as i32
}

fn CGFontGetLeading(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.line_gap_unscaled() as i32
}

fn CGFontGetFontBBox(env: &mut Environment, font: CGFontRef) -> CGRect {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    let (x_min, y_min, x_max, y_max) = font.global_bounding_box();
    assert!(x_min <= x_max && y_min <= y_max);
    CGRect {
        origin: CGPoint {
            x: x_min as CGFloat,
            y: y_min as CGFloat,
        },
        size: CGSize {
            width: (x_max - x_min) as CGFloat,
            height: (y_max - y_min) as CGFloat,
        },
    }
}

fn CGFontGetGlyphAdvances(
    env: &mut Environment,
    font: CGFontRef,
    glyphs: ConstPtr<CGGlyph>,
    count: GuestUSize,
    advances: MutPtr<i32>,
) -> bool {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    for i in 0..count {
        let glyph_id = env.mem.read(glyphs + i);
        let advance_width = font.glyph_hor_advance(glyph_id).unwrap().into();
        env.mem.write(advances + i, advance_width);
    }
    true
}

fn CGFontGetGlyphBBoxes(
    env: &mut Environment,
    font: CGFontRef,
    glyphs: ConstPtr<CGGlyph>,
    count: GuestUSize,
    boxes: MutPtr<CGRect>,
) -> bool {
    let mut res = true;
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    for i in 0..count {
        let glyph_id = env.mem.read(glyphs + i);
        let Some((x_min, y_min, x_max, y_max)) = font.glyph_bounding_box(glyph_id) else {
            res = false;
            // TODO: not sure what real device does here?
            env.mem.write(boxes + i, CGRectZero);
            continue;
        };
        assert!(x_min <= x_max && y_min <= y_max);
        // TODO: extract to a helper
        let rect = CGRect {
            origin: CGPoint {
                x: x_min as CGFloat,
                y: y_min as CGFloat,
            },
            size: CGSize {
                width: (x_max - x_min) as CGFloat,
                height: (y_max - y_min) as CGFloat,
            },
        };
        env.mem.write(boxes + i, rect);
    }
    res
}

fn CGFontGetItalicAngle(env: &mut Environment, font: CGFontRef) -> CGFloat {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.italic_angle().unwrap_or(0.0)
}

fn CGFontCopyTableForTag(env: &mut Environment, font: CGFontRef, tag: u32) -> CFDataRef {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    let table_data = font.table_data(tag).unwrap();

    let len = table_data.len() as GuestUSize;
    let guest_bytes = env.mem.alloc(len);
    env.mem
        .bytes_at_mut(guest_bytes.cast(), len)
        .copy_from_slice(table_data);
    let new: id = msg_class![env; NSData alloc];
    msg![env; new initWithBytesNoCopy:guest_bytes length:len]
}

fn CGFontGetNumberOfGlyphs(env: &mut Environment, font: CGFontRef) -> GuestUSize {
    let font = &env.objc.borrow::<CGFontHostObject>(font).font;
    font.number_of_glyphs().into()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGFontCreateWithDataProvider(_)),
    export_c_func!(CGFontRetain(_)),
    export_c_func!(CGFontRelease(_)),
    export_c_func!(CGFontGetGlyphsForUnichars(_, _, _, _)),
    export_c_func!(CGFontGetUnitsPerEm(_)),
    export_c_func!(CGFontGetAscent(_)),
    export_c_func!(CGFontGetDescent(_)),
    export_c_func!(CGFontGetLeading(_)),
    export_c_func!(CGFontGetFontBBox(_)),
    export_c_func!(CGFontGetGlyphAdvances(_, _, _, _)),
    export_c_func!(CGFontGetGlyphBBoxes(_, _, _, _)),
    export_c_func!(CGFontGetItalicAngle(_)),
    export_c_func!(CGFontCopyTableForTag(_, _)),
    export_c_func!(CGFontGetNumberOfGlyphs(_)),
];
