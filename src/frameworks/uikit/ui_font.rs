/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIFont`.

use super::ui_graphics::UIGraphicsGetCurrentContext;
use crate::font::{Font, TextAlignment, WrapMode};
use crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextDrawer;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};
use crate::Environment;

#[derive(Default)]
pub(super) struct State {
    regular: Option<Font>,
    bold: Option<Font>,
    italic: Option<Font>,
    regular_ja: Option<Font>,
    bold_ja: Option<Font>,
}

#[derive(Copy, Clone)]
enum FontKind {
    Regular,
    Bold,
    Italic,
}

struct UIFontHostObject {
    size: CGFloat,
    kind: FontKind,
}
impl HostObject for UIFontHostObject {}

/// Line break mode.
///
/// This is put here for convenience since it's font-related.
/// Apple puts it in its own header, also in UIKit.
pub type UILineBreakMode = NSInteger;
pub const UILineBreakModeWordWrap: UILineBreakMode = 0;
pub const UILineBreakModeCharacterWrap: UILineBreakMode = 1;
#[allow(dead_code)]
pub const UILineBreakModeClip: UILineBreakMode = 2;
#[allow(dead_code)]
pub const UILineBreakModeHeadTruncation: UILineBreakMode = 3;
pub const UILineBreakModeTailTruncation: UILineBreakMode = 4;
#[allow(dead_code)]
pub const UILineBreakModeMiddleTruncation: UILineBreakMode = 5;

/// Text alignment.
///
/// This is put here for convenience since it's font-related.
/// Apple puts it in its own header, also in UIKit.
pub type UITextAlignment = NSInteger;
pub const UITextAlignmentLeft: UITextAlignment = 0;
pub const UITextAlignmentCenter: UITextAlignment = 1;
pub const UITextAlignmentRight: UITextAlignment = 2;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIFont: NSObject

+ (id)systemFontOfSize:(CGFloat)size {
    // Cache for later use
    if env.framework_state.uikit.ui_font.regular.is_none() {
        env.framework_state.uikit.ui_font.regular = Some(Font::sans_regular());
    }
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::Regular,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)boldSystemFontOfSize:(CGFloat)size {
    // Cache for later use
    if env.framework_state.uikit.ui_font.bold.is_none() {
        env.framework_state.uikit.ui_font.bold = Some(Font::sans_bold());
    }
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::Bold,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)italicSystemFontOfSize:(CGFloat)size {
    // Cache for later use
    if env.framework_state.uikit.ui_font.italic.is_none() {
        env.framework_state.uikit.ui_font.italic = Some(Font::sans_italic());
    }
    let host_object = UIFontHostObject {
        size,
        kind: FontKind::Italic,
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

@end

};

fn convert_line_break_mode(ui_mode: UILineBreakMode) -> WrapMode {
    match ui_mode {
        UILineBreakModeWordWrap => WrapMode::Word,
        UILineBreakModeCharacterWrap => WrapMode::Char,
        // TODO: support this properly; fake support is so that UILabel works,
        // which has this as its default line break mode
        UILineBreakModeTailTruncation => WrapMode::Word,
        _ => unimplemented!("TODO: line break mode {}", ui_mode),
    }
}

#[rustfmt::skip]
fn get_font<'a>(state: &'a mut State, kind: FontKind, text: &str) -> &'a Font {
    // The default fonts (see font.rs) are the Liberation family, which are a
    // good substitute for Helvetica, the iPhone OS system font. Unfortunately,
    // there is no CJK support in these fonts. To support Super Monkey Ball in
    // Japanese, let's fall back to Noto Sans JP when necessary.
    // FIXME: This heuristic is incomplete and a proper font fallback system
    // should be used instead.
    for c in text.chars() {
        let c = c as u32;
        if (0x3000..=0x30FF).contains(&c) || // JA punctuation, kana
           (0xFF00..=0xFFEF).contains(&c) || // full-width/half-width chars
           (0x4e00..=0x9FA0).contains(&c) || // various kanji
           (0x3400..=0x4DBF).contains(&c) { // more kanji
            match kind {
                // CJK has no italic equivalent
                FontKind::Regular | FontKind::Italic => {
                    if state.regular_ja.is_none() {
                        state.regular_ja = Some(Font::sans_regular_ja());
                    }
                    return state.regular_ja.as_ref().unwrap();
                },
                FontKind::Bold => {
                    if state.bold_ja.is_none() {
                        state.bold_ja = Some(Font::sans_bold_ja());
                    }
                    return state.bold_ja.as_ref().unwrap();
                },
            }
        }
    }

    match kind {
        FontKind::Regular => state.regular.as_ref().unwrap(),
        FontKind::Bold => state.bold.as_ref().unwrap(),
        FontKind::Italic => state.italic.as_ref().unwrap(),
    }
}

/// Called by the `sizeWithFont:` method family on `NSString`.
pub fn size_with_font(
    env: &mut Environment,
    font: id,
    text: &str,
    constrained: Option<(CGSize, UILineBreakMode)>,
) -> CGSize {
    if text == " " {
        // ' ' will return 0 size, which is wrong
        // thus, we choose another similar char to calculate the size
        // (choice of '-' is made a bit arbitrary, just to look good on the screen)
        return size_with_font(env, font, "-", constrained);
    }

    let host_object = env.objc.borrow::<UIFontHostObject>(font);

    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );

    let wrap = constrained.map(|(size, ui_mode)| (size.width, convert_line_break_mode(ui_mode)));

    let (width, height) = font.calculate_text_size(host_object.size, text, wrap);

    CGSize { width, height }
}

/// Called by the `drawAtPoint:` method family on `NSString`.
pub fn draw_at_point(
    env: &mut Environment,
    font: id,
    text: &str,
    point: CGPoint,
    width_and_line_break_mode: Option<(CGFloat, UILineBreakMode)>,
) -> CGSize {
    let context = UIGraphicsGetCurrentContext(env);

    let host_object = env.objc.borrow::<UIFontHostObject>(font);

    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );

    let width_and_line_break_mode =
        width_and_line_break_mode.map(|(width, ui_mode)| (width, convert_line_break_mode(ui_mode)));
    let (width, height) =
        font.calculate_text_size(host_object.size, text, width_and_line_break_mode);

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);

    let fill_color = drawer.rgb_fill_color();

    let translation = drawer.translation();
    font.draw(
        host_object.size,
        text,
        (translation.0 + point.x, translation.1 + point.y),
        width_and_line_break_mode,
        TextAlignment::Left,
        |(x, y), coverage| {
            let (r, g, b, a) = fill_color;
            let (r, g, b, a) = (r * coverage, g * coverage, b * coverage, a * coverage);
            drawer.put_pixel((x, y), (r, g, b, a), /* blend: */ true);
        },
    );

    CGSize { width, height }
}

/// Called by the `drawInRect:` method family on `NSString`.
pub fn draw_in_rect(
    env: &mut Environment,
    font: id,
    text: &str,
    rect: CGRect,
    line_break_mode: UILineBreakMode,
    alignment: UITextAlignment,
) -> CGSize {
    let context = UIGraphicsGetCurrentContext(env);

    let text_size = size_with_font(env, font, text, Some((rect.size, line_break_mode)));

    let host_object = env.objc.borrow::<UIFontHostObject>(font);

    let font = get_font(
        &mut env.framework_state.uikit.ui_font,
        host_object.kind,
        text,
    );

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);

    let fill_color = drawer.rgb_fill_color();

    let (origin_x_offset, alignment) = match alignment {
        UITextAlignmentLeft => (0.0, TextAlignment::Left),
        UITextAlignmentCenter => (rect.size.width / 2.0, TextAlignment::Center),
        UITextAlignmentRight => (rect.size.width, TextAlignment::Right),
        _ => unimplemented!(),
    };

    let translation = drawer.translation();
    let scale = drawer.scale();
    assert_eq!(scale.0, 1.0);
    let scale_y = scale.1 as i32;
    assert_eq!(scale_y.abs(), 1);
    font.draw(
        host_object.size,
        text,
        (
            translation.0 + rect.origin.x + origin_x_offset,
            scale.1 * (translation.1 + rect.origin.y),
        ),
        Some((rect.size.width, convert_line_break_mode(line_break_mode))),
        alignment,
        |(x, y), coverage| {
            let (r, g, b, a) = fill_color;
            let (r, g, b, a) = (r * coverage, g * coverage, b * coverage, a * coverage);
            drawer.put_pixel((x, scale_y * y), (r, g, b, a), /* blend: */ true);
        },
    );

    text_size
}
