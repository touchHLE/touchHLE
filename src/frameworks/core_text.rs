/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CoreText.framework/CoreText`
//!
//! CoreText is the C-level text rendering API exposed by macOS and iOS
//! since iOS 3.2. We don't currently implement layout/glyph metrics
//! (apps that need full text shaping fall back to UIKit/UIFont), but
//! many apps reference the framework's exported string constants —
//! either as attribute keys when constructing font descriptor
//! dictionaries (`CTFontDescriptorCreateWithAttributes`) or as
//! attribute keys when building `CFAttributedStringRef` /
//! `NSAttributedString` instances for `CTFramesetterCreateWithAttributedString`
//! and friends — purely for `isEqual:` comparisons against keys
//! returned by the system.
//!
//! Per Apple's `CTFontDescriptor.h`, `CTFont.h`, `CTFontTraits.h` and
//! `CTStringAttributes.h` headers these are `CFStringRef` constants
//! with a canonical string value. For attributed-string attribute
//! keys most of them have the same value as their `NSAttributedString`
//! counterpart (e.g. `kCTFontAttributeName` == the `CFStringRef` for
//! the C string "NSFont", which is the same as the AppKit / UIKit
//! `NSFontAttributeName`), which is what makes CoreText / Foundation
//! attributed strings toll-free bridgeable. For touchHLE's purposes
//! the exact textual content only matters for identity comparisons;
//! we mirror the spelling Apple's public headers document.
//!
//! References:
//! - <https://developer.apple.com/documentation/coretext/font_descriptor_attribute_keys>
//! - <https://developer.apple.com/documentation/coretext/core_text_string_attributes>
//! - `CTFontDescriptor.h`, `CTFont.h`, `CTFontTraits.h`,
//!   `CTStringAttributes.h` (Apple SDK).

use crate::dyld::{ConstantExports, FunctionExports, HostConstant, HostDylib, export_c_func};
use crate::Environment;
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::{ConstVoidPtr, MutPtr};


/// Opaque CoreText font reference.
pub type CTFontRef = crate::objc::id;

/// `CTFontRef CTFontCreateWithGraphicsFont(CGFontRef graphicsFont,
///     CGFloat size, const CGAffineTransform *matrix,
///     CTFontDescriptorRef attributes)`
///
/// Creates a CTFont from a CGFont.  CoreText glyph layout and metric queries
/// are not currently implemented; returning NULL allows callers that check for
/// nil to fall back gracefully (typically to UIKit text rendering).
///
/// Reference: <https://developer.apple.com/documentation/coretext/1509694-ctfontcreatewithgraphicsfont>
fn CTFontCreateWithGraphicsFont(
    _env: &mut Environment,
    _graphics_font: crate::mem::ConstVoidPtr,  // CGFontRef (opaque)
    _size: CGFloat,
    _matrix: ConstVoidPtr,                       // const CGAffineTransform*
    _attributes: crate::objc::id,               // CTFontDescriptorRef
) -> CTFontRef {
    log_dbg!("CTFontCreateWithGraphicsFont: returning NULL (not implemented)");
    crate::objc::nil
}

/// `bool CTFontManagerRegisterGraphicsFont(CGFontRef font, CFErrorRef *error)`
///
/// Registers a graphics font so it can be looked up by name. touchHLE does not
/// maintain a CoreText font registry (text is rendered via UIKit/UIFont), but
/// the API contract is: return `true` on success and `false` with `*error`
/// set on failure. A valid (non-NULL) `CGFont` registers successfully; a NULL
/// font is the documented failure case.
///
/// Reference: <https://developer.apple.com/documentation/coretext/1499468-ctfontmanagerregistergraphicsfon>
fn CTFontManagerRegisterGraphicsFont(
    env: &mut Environment,
    font: ConstVoidPtr,    // CGFontRef
    error: MutPtr<crate::objc::id>, // CFErrorRef*
) -> bool {
    if !error.is_null() {
        env.mem.write(error, crate::objc::nil);
    }
    if font.is_null() {
        log_dbg!("CTFontManagerRegisterGraphicsFont: NULL font, returning false");
        return false;
    }
    log_dbg!("CTFontManagerRegisterGraphicsFont: accepting font, returning true");
    true
}

/// Opaque paragraph style reference.
/// On real iOS this is a CFType; we represent it as an opaque ObjC id allocated
/// via the runtime so it can participate in retain/release.
pub type CTParagraphStyleRef = crate::objc::id;

/// `CTParagraphStyleRef CTParagraphStyleCreate(
///     const CTParagraphStyleSetting *settings,
///     size_t settingCount)`
///
/// Creates an immutable paragraph style object from an array of
/// `CTParagraphStyleSetting` structures. Each setting specifies a paragraph
/// property (alignment, line spacing, indents, etc.).
///
/// touchHLE does not perform CoreText layout, so the returned object is an
/// opaque token that apps can store in attributed-string dictionaries and
/// pass to CTFramesetter without crashing. The individual settings are
/// intentionally ignored — visual paragraph formatting is not reproduced —
/// but returning a valid non-NULL object is critical for apps that assert
/// on the return value or use it as a dictionary key.
///
/// Reference: <https://developer.apple.com/documentation/coretext/1524171-ctparagraphstylecreate>
fn CTParagraphStyleCreate(
    env: &mut Environment,
    _settings: ConstVoidPtr, // const CTParagraphStyleSetting*
    _setting_count: u32,     // size_t (32-bit guest)
) -> CTParagraphStyleRef {
    // Allocate a minimal NSObject-based token. The object has no meaningful
    // internal state — it just needs to survive retain/release cycles and
    // respond to `isEqual:` / `hash` (NSObject defaults are fine for identity
    // semantics).
    let obj: CTParagraphStyleRef = crate::objc::msg_class![env; NSObject new];
    log_dbg!(
        "CTParagraphStyleCreate(settings={:?}, count={}) => {:?}",
        _settings,
        _setting_count,
        obj
    );
    // Return ownership to the caller (balanced by CFRelease or autorelease
    // pool drain on the guest side).
    obj
}

/// `CTParagraphStyleRef CTParagraphStyleCreateCopy(CTParagraphStyleRef paragraphStyle)`
///
/// Returns a copy of the paragraph style. Since our implementation carries no
/// mutable state, we simply retain and return the same object.
///
/// Reference: <https://developer.apple.com/documentation/coretext/1525098-ctparagraphstylecreatecopy>
fn CTParagraphStyleCreateCopy(
    env: &mut Environment,
    paragraph_style: CTParagraphStyleRef,
) -> CTParagraphStyleRef {
    if paragraph_style.is_null() {
        return crate::objc::nil;
    }
    crate::objc::retain(env, paragraph_style);
    paragraph_style
}

/// `bool CTParagraphStyleGetValueForSpecifier(
///     CTParagraphStyleRef paragraphStyle,
///     CTParagraphStyleSpecifier spec,
///     size_t valueBufferSize,
///     void *valueBuffer)`
///
/// Retrieves the value of a specific paragraph-style property. Since we don't
/// store settings, we zero-fill the output buffer and return false (indicating
/// the default value is being returned).
///
/// Reference: <https://developer.apple.com/documentation/coretext/1525353-ctparagraphstylegetvalueforspeci>
fn CTParagraphStyleGetValueForSpecifier(
    env: &mut Environment,
    _paragraph_style: CTParagraphStyleRef,
    _spec: u32,              // CTParagraphStyleSpecifier
    value_buffer_size: u32,  // size_t
    value_buffer: MutPtr<u8>,
) -> bool {
    // Zero-fill the buffer — this provides the documented "default" values
    // (alignment = natural, line break mode = word wrapping, indents = 0, etc.)
    if !value_buffer.is_null() && value_buffer_size > 0 {
        let slice = env.mem.bytes_at_mut(value_buffer, value_buffer_size);
        slice.fill(0);
    }
    false // "specifier not found" — caller uses default
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CTFontCreateWithGraphicsFont(_, _, _, _)),
    export_c_func!(CTFontManagerRegisterGraphicsFont(_, _)),
    export_c_func!(CTParagraphStyleCreate(_, _)),
    export_c_func!(CTParagraphStyleCreateCopy(_)),
    export_c_func!(CTParagraphStyleGetValueForSpecifier(_, _, _, _)),
];

pub const CONSTANTS: ConstantExports = &[
    // CTFontDescriptor.h
    (
        "_kCTFontNameAttribute",
        HostConstant::NSString("NSFontNameAttribute"),
    ),
    (
        "_kCTFontFamilyNameAttribute",
        HostConstant::NSString("NSFontFamilyAttribute"),
    ),
    (
        "_kCTFontStyleNameAttribute",
        HostConstant::NSString("NSFontFaceAttribute"),
    ),
    (
        "_kCTFontTraitsAttribute",
        HostConstant::NSString("NSCTFontTraitsAttribute"),
    ),
    (
        "_kCTFontURLAttribute",
        HostConstant::NSString("NSCTFontFileURLAttribute"),
    ),
    (
        "_kCTFontDisplayNameAttribute",
        HostConstant::NSString("NSFontVisibleNameAttribute"),
    ),
    (
        "_kCTFontSizeAttribute",
        HostConstant::NSString("NSFontSizeAttribute"),
    ),
    (
        "_kCTFontMatrixAttribute",
        HostConstant::NSString("NSCTFontMatrixAttribute"),
    ),
    (
        "_kCTFontCascadeListAttribute",
        HostConstant::NSString("NSCTFontCascadeListAttribute"),
    ),
    (
        "_kCTFontCharacterSetAttribute",
        HostConstant::NSString("NSCTFontCharacterSetAttribute"),
    ),
    (
        "_kCTFontLanguagesAttribute",
        HostConstant::NSString("NSCTFontLanguagesAttribute"),
    ),
    (
        "_kCTFontBaselineAdjustAttribute",
        HostConstant::NSString("NSCTFontBaselineAdjustAttribute"),
    ),
    (
        "_kCTFontMacintoshEncodingsAttribute",
        HostConstant::NSString("NSCTFontMacintoshEncodingsAttribute"),
    ),
    (
        "_kCTFontFeaturesAttribute",
        HostConstant::NSString("NSCTFontFeaturesAttribute"),
    ),
    (
        "_kCTFontFeatureSettingsAttribute",
        HostConstant::NSString("NSCTFontFeatureSettingsAttribute"),
    ),
    (
        "_kCTFontFixedAdvanceAttribute",
        HostConstant::NSString("NSCTFontFixedAdvanceAttribute"),
    ),
    (
        "_kCTFontOrientationAttribute",
        HostConstant::NSString("NSCTFontOrientationAttribute"),
    ),
    (
        "_kCTFontFormatAttribute",
        HostConstant::NSString("NSCTFontFormatAttribute"),
    ),
    (
        "_kCTFontRegistrationScopeAttribute",
        HostConstant::NSString("NSCTFontRegistrationScopeAttribute"),
    ),
    (
        "_kCTFontPriorityAttribute",
        HostConstant::NSString("NSCTFontPriorityAttribute"),
    ),
    (
        "_kCTFontEnabledAttribute",
        HostConstant::NSString("NSCTFontEnabledAttribute"),
    ),
    (
        "_kCTFontDownloadableAttribute",
        HostConstant::NSString("NSCTFontDownloadableAttribute"),
    ),
    (
        "_kCTFontDownloadedAttribute",
        HostConstant::NSString("NSCTFontDownloadedAttribute"),
    ),
    // CTFontTraits.h
    (
        "_kCTFontSymbolicTrait",
        HostConstant::NSString("NSCTFontSymbolicTrait"),
    ),
    (
        "_kCTFontWeightTrait",
        HostConstant::NSString("NSCTFontWeightTrait"),
    ),
    (
        "_kCTFontWidthTrait",
        HostConstant::NSString("NSCTFontWidthTrait"),
    ),
    (
        "_kCTFontSlantTrait",
        HostConstant::NSString("NSCTFontSlantTrait"),
    ),
    // CTStringAttributes.h — attribute keys for `CFAttributedStringRef`
    // (and, toll-free bridged, `NSAttributedString`) used by
    // `CTFramesetterCreateWithAttributedString` and friends.
    // Canonical string values come from Apple's public
    // `CTStringAttributes.h` header; many deliberately share their
    // value with the corresponding `NSAttributedString` attribute name
    // so the same dictionary can be used by both CoreText and
    // UIKit/AppKit.
    (
        "_kCTFontAttributeName",
        // Same value as `NSFontAttributeName`.
        HostConstant::NSString("NSFont"),
    ),
    (
        "_kCTForegroundColorAttributeName",
        HostConstant::NSString("CTForegroundColor"),
    ),
    (
        "_kCTForegroundColorFromContextAttributeName",
        HostConstant::NSString("CTForegroundColorFromContext"),
    ),
    (
        "_kCTBackgroundColorAttributeName",
        HostConstant::NSString("kCTBackgroundColorAttributeName"),
    ),
    (
        "_kCTKernAttributeName",
        // Same value as `NSKernAttributeName`.
        HostConstant::NSString("NSKern"),
    ),
    (
        "_kCTLigatureAttributeName",
        // Same value as `NSLigatureAttributeName`.
        HostConstant::NSString("NSLigature"),
    ),
    (
        "_kCTParagraphStyleAttributeName",
        // Same value as `NSParagraphStyleAttributeName`.
        HostConstant::NSString("NSParagraphStyle"),
    ),
    (
        "_kCTStrokeWidthAttributeName",
        // Same value as `NSStrokeWidthAttributeName`.
        HostConstant::NSString("NSStrokeWidth"),
    ),
    (
        "_kCTStrokeColorAttributeName",
        // Same value as `NSStrokeColorAttributeName`.
        HostConstant::NSString("NSStrokeColor"),
    ),
    (
        "_kCTUnderlineStyleAttributeName",
        HostConstant::NSString("CTUnderlineStyle"),
    ),
    (
        "_kCTUnderlineColorAttributeName",
        HostConstant::NSString("CTUnderlineColor"),
    ),
    (
        "_kCTSuperscriptAttributeName",
        // Same value as `NSSuperscriptAttributeName`.
        HostConstant::NSString("NSSuperScript"),
    ),
    (
        "_kCTVerticalFormsAttributeName",
        HostConstant::NSString("CTVerticalForms"),
    ),
    (
        "_kCTGlyphInfoAttributeName",
        HostConstant::NSString("CTGlyphInfo"),
    ),
    (
        "_kCTCharacterShapeAttributeName",
        // Same value as `NSCharacterShapeAttributeName`.
        HostConstant::NSString("NSCharacterShape"),
    ),
    (
        "_kCTLanguageAttributeName",
        HostConstant::NSString("CTLanguage"),
    ),
    (
        "_kCTRunDelegateAttributeName",
        HostConstant::NSString("CTRunDelegate"),
    ),
    (
        "_kCTBaselineClassAttributeName",
        HostConstant::NSString("CTBaselineClass"),
    ),
    (
        "_kCTBaselineInfoAttributeName",
        HostConstant::NSString("CTBaselineInfo"),
    ),
    (
        "_kCTBaselineReferenceInfoAttributeName",
        HostConstant::NSString("CTBaselineReferenceInfo"),
    ),
    (
        "_kCTBaselineOffsetAttributeName",
        // Same value as `NSBaselineOffsetAttributeName`.
        HostConstant::NSString("NSBaselineOffset"),
    ),
    (
        "_kCTWritingDirectionAttributeName",
        // Same value as `NSWritingDirectionAttributeName`.
        HostConstant::NSString("NSWritingDirection"),
    ),
    (
        "_kCTTrackingAttributeName",
        HostConstant::NSString("CTTracking"),
    ),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreText.framework/CoreText",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};
