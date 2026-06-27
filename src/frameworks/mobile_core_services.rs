/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `MobileCoreServices.framework/MobileCoreServices`.
//!
//! On iOS, MobileCoreServices is the umbrella for Uniform Type Identifier
//! types and constants (`UTType*`, `kUTType*`). It is pulled in transitively
//! by all sorts of high-level UIKit APIs — `UIDocumentInteractionController`,
//! `MFMailComposeViewController`, `UIImagePickerController` — so most apps
//! end up listing it in their Mach-O dependency table even when they never
//! call any UTType function directly.
//!
//! Without a [crate::dyld::HostDylib] entry for the path
//! `/System/Library/Frameworks/MobileCoreServices.framework/`
//! `MobileCoreServices`,
//! touchHLE prints a `Warning: app binary depends on unimplemented or missing
//! dylib …` at startup, which can spook users into reporting otherwise-fine
//! apps as broken (e.g. HyperHLE appdb report #23, Mutant Fridge).
//!
//! This stub exists so the dependency is recognized and the warning is
//! suppressed. Real UTType handling is not implemented; functions that
//! actually need to consult UTType data should be added here as they come up.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};

pub const FUNCTIONS: FunctionExports = &[];

// kUTTagClass* / kUTType* are CFStringRef constants exported by
// MobileCoreServices. Apps reference them in `__nl_symbol_ptr` via the
// UTType conversion API even when they never call those functions at
// runtime (e.g. UIImagePickerController media-type checks). Exporting them
// as plain CFString placeholders silences the non-lazy-symbol warning and
// keeps key-comparison-by-identity working.
pub const CONSTANTS: ConstantExports = &[
    (
        "_kUTTagClassFilenameExtension",
        HostConstant::NSString("public.filename-extension"),
    ),
    (
        "_kUTTagClassMIMEType",
        HostConstant::NSString("public.mime-type"),
    ),
    (
        "_kUTTagClassNSPboardType",
        HostConstant::NSString("com.apple.nspboard-type"),
    ),
    (
        "_kUTTagClassOSType",
        HostConstant::NSString("com.apple.ostype"),
    ),
    // The few UTType identifiers most commonly passed by name.
    ("_kUTTypeImage", HostConstant::NSString("public.image")),
    ("_kUTTypeMovie", HostConstant::NSString("public.movie")),
    ("_kUTTypeVideo", HostConstant::NSString("public.video")),
    ("_kUTTypeAudio", HostConstant::NSString("public.audio")),
    ("_kUTTypeData", HostConstant::NSString("public.data")),
    ("_kUTTypeURL", HostConstant::NSString("public.url")),
    ("_kUTTypeText", HostConstant::NSString("public.text")),
    (
        "_kUTTypePlainText",
        HostConstant::NSString("public.plain-text"),
    ),
    ("_kUTTypeJPEG", HostConstant::NSString("public.jpeg")),
    ("_kUTTypePNG", HostConstant::NSString("public.png")),
    // Additional UTType identifiers covering the common image, audio,
    // video and document formats Apple ships with the OS. Values are
    // taken from <https://developer.apple.com/documentation/uniformtypeidentifiers/uttype/system-declared-uniform-type-identifiers>.
    ("_kUTTypeGIF", HostConstant::NSString("com.compuserve.gif")),
    ("_kUTTypeTIFF", HostConstant::NSString("public.tiff")),
    ("_kUTTypeBMP", HostConstant::NSString("com.microsoft.bmp")),
    ("_kUTTypeICO", HostConstant::NSString("com.microsoft.ico")),
    (
        "_kUTTypeJPEG2000",
        HostConstant::NSString("public.jpeg-2000"),
    ),
    (
        "_kUTTypeAppleICNS",
        HostConstant::NSString("com.apple.icns"),
    ),
    (
        "_kUTTypeQuickTimeImage",
        HostConstant::NSString("com.apple.quicktime-image"),
    ),
    (
        "_kUTTypeQuickTimeMovie",
        HostConstant::NSString("com.apple.quicktime-movie"),
    ),
    ("_kUTTypeMPEG", HostConstant::NSString("public.mpeg")),
    (
        "_kUTTypeMPEG4",
        HostConstant::NSString("public.mpeg-4"),
    ),
    (
        "_kUTTypeMPEG4Audio",
        HostConstant::NSString("public.mpeg-4-audio"),
    ),
    (
        "_kUTTypeAppleProtectedMPEG4Audio",
        HostConstant::NSString("com.apple.protected-mpeg-4-audio"),
    ),
    (
        "_kUTTypeAppleProtectedMPEG4Video",
        HostConstant::NSString("com.apple.protected-mpeg-4-video"),
    ),
    (
        "_kUTTypeMP3",
        HostConstant::NSString("public.mp3"),
    ),
    (
        "_kUTTypeAVIMovie",
        HostConstant::NSString("public.avi"),
    ),
    (
        "_kUTTypeAudiovisualContent",
        HostConstant::NSString("public.audiovisual-content"),
    ),
    (
        "_kUTTypeWaveformAudio",
        HostConstant::NSString("com.microsoft.waveform-audio"),
    ),
    (
        "_kUTTypeAIFFAudio",
        HostConstant::NSString("public.aiff-audio"),
    ),
    (
        "_kUTTypeAUAudio",
        HostConstant::NSString("public.au-audio"),
    ),
    (
        "_kUTTypeMIDIAudio",
        HostConstant::NSString("public.midi-audio"),
    ),
    // Content/format/abstract types.
    ("_kUTTypeContent", HostConstant::NSString("public.content")),
    ("_kUTTypeItem", HostConstant::NSString("public.item")),
    (
        "_kUTTypeArchive",
        HostConstant::NSString("public.archive"),
    ),
    (
        "_kUTTypeFolder",
        HostConstant::NSString("public.folder"),
    ),
    (
        "_kUTTypeDirectory",
        HostConstant::NSString("public.directory"),
    ),
    (
        "_kUTTypeApplication",
        HostConstant::NSString("com.apple.application"),
    ),
    (
        "_kUTTypeBundle",
        HostConstant::NSString("com.apple.bundle"),
    ),
    (
        "_kUTTypeFramework",
        HostConstant::NSString("com.apple.framework"),
    ),
    (
        "_kUTTypeExecutable",
        HostConstant::NSString("public.executable"),
    ),
    // Document types.
    ("_kUTTypeUTF8PlainText", HostConstant::NSString("public.utf8-plain-text")),
    (
        "_kUTTypeUTF16PlainText",
        HostConstant::NSString("public.utf16-plain-text"),
    ),
    ("_kUTTypeRTF", HostConstant::NSString("public.rtf")),
    ("_kUTTypeHTML", HostConstant::NSString("public.html")),
    ("_kUTTypeXML", HostConstant::NSString("public.xml")),
    ("_kUTTypeJSON", HostConstant::NSString("public.json")),
    (
        "_kUTTypePropertyList",
        HostConstant::NSString("com.apple.property-list"),
    ),
    (
        "_kUTTypeXMLPropertyList",
        HostConstant::NSString("com.apple.xml-property-list"),
    ),
    (
        "_kUTTypeBinaryPropertyList",
        HostConstant::NSString("com.apple.binary-property-list"),
    ),
    ("_kUTTypePDF", HostConstant::NSString("com.adobe.pdf")),
    (
        "_kUTTypeVCard",
        HostConstant::NSString("public.vcard"),
    ),
    // VCS / source / build artefacts occasionally referenced by IDE-style apps.
    (
        "_kUTTypeSourceCode",
        HostConstant::NSString("public.source-code"),
    ),
    (
        "_kUTTypeCSource",
        HostConstant::NSString("public.c-source"),
    ),
    (
        "_kUTTypeObjectiveCSource",
        HostConstant::NSString("public.objective-c-source"),
    ),
    // `kUTTypeTagSpecificationKey` is the dictionary key used by
    // `UTTypeCreateAllIdentifiersForTag()` / `UTTypeCopyDeclaration()`
    // to look up the type's tag specification (filename extensions,
    // MIME types, etc.). The literal value matches Apple's headers.
    // <https://developer.apple.com/documentation/coreservices/kuttypetagspecificationkey>
    (
        "_kUTTypeTagSpecificationKey",
        HostConstant::NSString("UTTypeTagSpecification"),
    ),
];
