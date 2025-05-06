use owned_ttf_parser::GlyphId;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::frameworks::core_foundation::cf_string::CFStringRef;
use crate::frameworks::core_foundation::{CFIndex, CFTypeRef};
use crate::objc::{id, ClassExports, HostObject};
use crate::{export_c_func, objc_classes};
use crate::font::{Font, TextAlignment};
use crate::frameworks::core_foundation::cf_allocator::kCFAllocatorDefault;
use crate::frameworks::core_foundation::cf_data::{CFDataCreate, CFDataRef};
use crate::frameworks::core_graphics::cg_data_provider::CGDataProviderRef;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::core_graphics::cg_bitmap_context::CGBitmapContextDrawer;
use crate::frameworks::core_graphics::cg_context::CGContextHostObject;
use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
pub mod CMap {
    use symphonia::core::meta::StandardTagKey::EncodedBy;
    use crate::font::Font;

    #[derive(Clone)]
    pub struct Table {
        pub version: u16,
        pub subtable_count: u16,
        pub encoding_records: Vec<EncodingRecord>,
        pub subtables: Vec<Subtable>,
    }

    #[derive(Clone)]
    pub struct EncodingRecord {
        pub platform_id: u16,
        pub platform_specific_id: u16,
        pub offset: u32,
    }

    #[derive(Clone)]
    pub struct SubtableFormat6 {
        pub format: u16,
        pub length: u16,
        pub language: u16,
        pub first_code: u16,
        pub entry_count: u16,
        pub glyph_index_array: Vec<u16>,
    }

    #[derive(Clone)]
    pub enum Subtable {
        Format6(SubtableFormat6)
    }

    pub fn table_from_font(_font: &Font) -> Table {
        let ascii_range = (0u8..255u8);
        let codepoints: Vec<u16> = ascii_range
            .map(|cp| cp as u16)
            .collect();
        Table {
            version: 0,
            subtable_count: 1,
            encoding_records: [
                EncodingRecord {
                    platform_id: 0,
                    platform_specific_id: 3,
                    offset: (size_of::<u16>()*2 + size_of::<EncodingRecord>()) as u32
                }
            ].to_vec(),
            subtables: [
                Subtable::Format6(
                    SubtableFormat6{
                        format: 6,
                        length: 256*2,
                        language: 0,
                        first_code: 0,
                        entry_count: codepoints.len() as u16,
                        glyph_index_array: codepoints
                    }
                )
            ].to_vec()
        }
    }

    pub fn serialize(table: &Table) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&table.version.to_be_bytes());
        buf.extend_from_slice(&table.subtable_count.to_be_bytes());
        for record in table.encoding_records.iter() {
            buf.extend_from_slice(&record.platform_id.to_be_bytes());
            buf.extend_from_slice(&record.platform_specific_id.to_be_bytes());
            buf.extend_from_slice(&record.offset.to_be_bytes());
        }
        for subtable in table.subtables.iter() {
            match subtable {
                Subtable::Format6(format6) => {
                    buf.extend_from_slice(&format6.format.to_be_bytes());
                    buf.extend_from_slice(&format6.length.to_be_bytes());
                    buf.extend_from_slice(&format6.language.to_be_bytes());
                    buf.extend_from_slice(&format6.first_code.to_be_bytes());
                    buf.extend_from_slice(&format6.entry_count.to_be_bytes());
                    for &glyph_id in &format6.glyph_index_array {
                        buf.extend_from_slice(&glyph_id.to_be_bytes());
                    }
                }
            }
        }
        buf
    }
}
pub type CGFontRef = CFTypeRef;
pub struct CGFontHostObject {
    font: Font,
}
impl HostObject for CGFontHostObject {}
pub type CGFontIndex = u16;
pub type CGGlyph = CGFontIndex;
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CGFont: NSObject
@end

};

/// Called by `crate::frameworks::core_graphics::cg_context::CGContextShowGlyphsAtPoint`.
pub fn glyphs_at_point(
    env: &mut Environment,
    font: id,
    glyphs: &[GlyphId],
    point: CGPoint,
) {
    let context = UIGraphicsGetCurrentContext(env);

    let host_object = env.objc.borrow::<CGFontHostObject>(font);

    let font = &host_object.font;
    let context_host = env.objc.borrow::<CGContextHostObject>(context);

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let fill_color = drawer.rgb_fill_color();

    font.draw_glyphs(
        context_host.state.font_size * 0.5,
        glyphs,
        (point.x, point.y),
        None,
        TextAlignment::Left,
        |raster_glyph| {
            crate::frameworks::uikit::ui_font::draw_font_glyph(
                &mut drawer,
                raster_glyph,
                fill_color,
                /* clip_x: */ None,
                /* clip_y: */ None,
            )
        },
    );
}

pub fn CGFontCreateWithFontName(env: &mut Environment, name: CFStringRef) -> CGFontRef {
    let host_obj = Box::new(CGFontHostObject {
        font: Font::sans_regular() // TODO actually load requested font.
    });
    let name = to_rust_string(env, name);
    log!("Ignoring CGFontCreateWithFontName((CFString*){:?})", name);
    let class = env.objc.get_known_class("CGFont", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

pub fn CGFontCreateWithDataProvider(env: &mut Environment, provider: CGDataProviderRef) -> CGFontRef {
    log!("Ignoring CGFontCreateWithDataProvider((CGDataProvider*){:?})", provider);
    let host_obj = Box::new(CGFontHostObject {
        font: Font::sans_regular() // TODO actually load requested font.
    });
    let class = env.objc.get_known_class("CGFont", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

pub fn CGFontCopyTableForTag(env: &mut Environment, font: CGFontRef, tag: u32) -> CFDataRef {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    let table_bytes = font.font.get_raw_table(tag);
    let table_bytes_guest = env.mem.alloc(table_bytes.len() as GuestUSize).cast();
    for i in 0..table_bytes.len() {
        env.mem.write::<u8>(table_bytes_guest + i as GuestUSize, table_bytes[i]);
    }
    CFDataCreate(env, kCFAllocatorDefault, table_bytes_guest.cast_const(), table_bytes.len() as CFIndex)
}

pub fn CGFontCopyFullName(env: &mut Environment, font: CGFontRef) -> CFStringRef {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    from_rust_string(env, font.font.name())
}

pub fn CGFontGetGlyphBBoxes(env: &mut Environment, font: CGFontRef, glyphs: ConstPtr<CGGlyph>, count: GuestUSize, bboxes: MutPtr<CGRect>) -> bool {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    let mut point = CGPoint {
        x: 0.0,
        y: 0.0,
    };

    for (x, i) in (1..count).map(|i| (glyphs + i, i)).map(|(ptr, i)| (env.mem.read(ptr), i)).collect::<Vec<_>>() {
        let bbox = font.font.glyph_size(GlyphId(x as u16));
        let width = bbox.width() as f32;
        let height = bbox.height() as f32;
        let rect = CGRect {
            origin: point,
            size: CGSize {
                width,
                height,
            }
        };
        env.mem.write(bboxes + i, rect);
        point.x += width;
    }
    true
}

pub fn CGFontGetGlyphAdvances(env: &mut Environment, font: CGFontRef, glyphs: ConstPtr<CGGlyph>, count: GuestUSize, advances: MutPtr<i32>) -> bool {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    for (x, i) in (1..count).map(|i| (glyphs + i, i)).map(|(ptr, i)| (env.mem.read(ptr), i)).collect::<Vec<_>>() {
        env.mem.write(advances + i, font.font.advance_unscaled(x));
    }
    true
}

pub fn CGFontGetAscent(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    font.font.ascent_unscaled()
}

pub fn CGFontGetDescent(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    font.font.descent_unscaled()
}
pub fn CGFontGetCapHeight(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    font.font.cap_height()
}

pub fn CGFontGetUnitsPerEm(env: &mut Environment, font: CGFontRef) -> i32 {
    let font = env.objc.borrow::<CGFontHostObject>(font);
    font.font.units_per_em() as i32
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGFontCopyFullName(_)),
    export_c_func!(CGFontCreateWithFontName(_)),
    export_c_func!(CGFontCreateWithDataProvider(_)),
    export_c_func!(CGFontCopyTableForTag(_, _)),
    export_c_func!(CGFontGetGlyphBBoxes(_, _, _, _)),
    export_c_func!(CGFontGetGlyphAdvances(_, _, _, _)),
    export_c_func!(CGFontGetAscent(_)),
    export_c_func!(CGFontGetDescent(_)),
    export_c_func!(CGFontGetUnitsPerEm(_)),
    export_c_func!(CGFontGetCapHeight(_)),
];