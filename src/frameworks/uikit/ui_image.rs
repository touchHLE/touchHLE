/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImage`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_graphics::cg_context::CGContextDrawImage;
use crate::frameworks::core_graphics::cg_image::{
    self, CGImageGetHeight, CGImageGetWidth, CGImageRef, CGImageRelease, CGImageRetain,
};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_data, ns_string, NSInteger};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::fs::GuestPath;
use crate::image::Image;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, SEL,
};
use crate::{Environment, paths};
use std::collections::HashMap;

const CACHE_SIZE: usize = 10;

#[derive(Default)]
pub struct State {
    /// Cache of images for `[UIImage imageNamed:]` method.
    /// Images are explicitly retained.
    cached_images: HashMap<String, id>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.framework_state.uikit.ui_image
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.uikit.ui_image
    }
}

struct UIImageHostObject {
    cg_image: CGImageRef,
}
impl HostObject for UIImageHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImage: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIImageHostObject { cg_image: nil });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)imageWithCGImage:(CGImageRef)cg_image {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCGImage:cg_image];
    autorelease(env, new)
}

+ (id)imageNamed:(id)name { // NSString*
    // TODO: figure out whether this is actually correct in all cases
    let bundle: id = msg_class![env; NSBundle mainBundle];
    let path: id = msg![env; bundle pathForResource:name ofType:nil];
    let name_str = ns_string::to_rust_string(env, name).to_string();
    if path == nil {
        log!("Warning: [UIImage imageNamed:{:?}] => nil", name_str);
        return nil;
    }
    // TODO: find a better eviction policy
    if State::get(env).cached_images.len() > CACHE_SIZE {
        let cache = std::mem::take(&mut State::get_mut(env).cached_images);
        log_dbg!("Evicting {} images from UIImage cache.", cache.len());
        for (_, img) in cache {
            release(env, img);
        }
    }
    if !State::get(env).cached_images.contains_key(&name_str) {
        let img = msg![env; this imageWithContentsOfFile:path];
        retain(env, img);
        State::get_mut(env).cached_images.insert(name_str.clone(), img);
    }
    *State::get(env).cached_images.get(&name_str).unwrap()
}

+ (id)imageWithContentsOfFile:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfFile:path];
    autorelease(env, new)
}

+ (id)imageWithData:(id)data { // NSData*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithData:data];
    autorelease(env, new)
}

- (())dealloc {
    let &UIImageHostObject { cg_image } = env.objc.borrow(this);
    CGImageRelease(env, cg_image);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithCGImage:(CGImageRef)cg_image {
    CGImageRetain(env, cg_image);
    env.objc.borrow_mut::<UIImageHostObject>(this).cg_image = cg_image;
    this
}

- (id)initWithContentsOfFile:(id)path { // NSString*
    if path == nil {
        return nil;
    }
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    let Ok(bytes) = env.fs.read(GuestPath::new(&path)) else {
        log!("Warning: couldn't read image file at {:?}, returning nil", path);
        release(env, this);
        return nil;
    };
    // TODO: Real error handling. For now, most errors are likely to be caused
    //       by a functionality gap in touchHLE, not the app actually trying to
    //       load a broken file, so panicking is most useful.
    let image = Image::from_bytes(&bytes).unwrap();
    let cg_image = cg_image::from_image(env, image);
    env.objc.borrow_mut::<UIImageHostObject>(this).cg_image = cg_image;
    this
}

- (id)initWithData:(id)data { // NSData*
    let slice = ns_data::to_rust_slice(env, data);
    // TODO: refactor common parts
    let image = Image::from_bytes(slice).unwrap();
    let cg_image = cg_image::from_image(env, image);
    env.objc.borrow_mut::<UIImageHostObject>(this).cg_image = cg_image;
    this
}

- (id) stretchableImageWithLeftCapWidth:(NSInteger)_leftCapWidth topCapHeight:(NSInteger)_topCapHeight {
    log!("TODO: properly support stretchableImageWithLeftCapWidth:topCapHeight:");
    retain(env, this)
}

// TODO: more init methods
// TODO: more accessors

- (CGImageRef)CGImage {
    env.objc.borrow::<UIImageHostObject>(this).cg_image
}

// TODO: should have UIImageOrientation type
- (NSInteger)imageOrientation {
    // FIXME: load image orientation info from file?
    0 // UIImageOrientationUp
}

- (CGSize)size {
    let image = env.objc.borrow::<UIImageHostObject>(this).cg_image;
    let (width, height) = cg_image::borrow_image(&env.objc, image).dimensions();
    CGSize {
        width: width as _,
        height: height as _,
    }
}

- (())drawInRect:(CGRect)rect {
    let context = UIGraphicsGetCurrentContext(env);
    let image = env.objc.borrow::<UIImageHostObject>(this).cg_image;
    CGContextDrawImage(env, context, rect, image);
}

- (())drawAtPoint:(CGPoint)point {
    let context = UIGraphicsGetCurrentContext(env);
    let image = env.objc.borrow::<UIImageHostObject>(this).cg_image;
    let rect = CGRect {
        origin: point,
        size: CGSize {
            width: CGImageGetWidth(env, image) as CGFloat,
            height: CGImageGetHeight(env, image) as CGFloat,
        }
    };
    CGContextDrawImage(env, context, rect, image);
}

@end

};

fn png_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn png_adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

fn png_zlib_compress_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // zlib header: CMF/FLG for deflate, 32K window, "no compression" tuning.
    out.push(0x78);
    out.push(0x01);

    let mut remaining = data;
    while !remaining.is_empty() {
        let block_len = remaining.len().min(0xFFFF);
        let is_last = block_len == remaining.len();
        let bfinal: u8 = if is_last { 1 } else { 0 };

        // BFINAL in bit 0, BTYPE=00 (stored) in bits 1–2 -> 0x00 or 0x01.
        out.push(bfinal);

        let len = block_len as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&remaining[..block_len]);

        remaining = &remaining[block_len..];
    }

    let adler = png_adler32(data);
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

fn encode_png_rgba8(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    let mut out = Vec::new();
    out.extend_from_slice(&PNG_SIG);

    fn write_chunk(buf: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        let len = data.len() as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(&kind);
        buf.extend_from_slice(data);

        let mut crc_input = Vec::with_capacity(4 + data.len());
        crc_input.extend_from_slice(&kind);
        crc_input.extend_from_slice(data);
        let crc = png_crc32(&crc_input);
        buf.extend_from_slice(&crc.to_be_bytes());
    }

    // IHDR
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut out, *b"IHDR", &ihdr);

    // Raw image data: each row = filter byte 0 + RGBA pixels
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in 0..(height as usize) {
        raw.push(0); // filter type 0 (None)
        let start = row * stride;
        raw.extend_from_slice(&rgba[start..start + stride]);
    }

    // Compress and write IDAT
    let compressed = png_zlib_compress_stored(&raw);
    write_chunk(&mut out, *b"IDAT", &compressed);

    // IEND
    write_chunk(&mut out, *b"IEND", &[]);

    out
}

fn UIImageWriteToSavedPhotosAlbum(
    env: &mut Environment,
    image: id,
    completionTarget: id,
    completionSelector: SEL,
    contextInfo: id,
) {
    log_dbg!(
        "UIImageWriteToSavedPhotosAlbum image:{:?} completionTarget:{:?} completionSelector:{:?}",
        image,
        completionTarget,
        completionSelector,
    );

    if image != nil {
        // Get underlying CGImage from UIImage
        let cg_image: CGImageRef = msg![env; image CGImage];
        if cg_image != nil {
            let img = cg_image::borrow_image(&env.objc, cg_image);
            let (w, h) = img.dimensions();
            let rgba = img.pixels();

            // Encode PNG in-memory
            let png_data = encode_png_rgba8(w, h, rgba);

            let base = paths::user_data_base_path();
            let joined_photo_album_dir = base.join(paths::PHOTO_ALBUM_DIR);

            if let Err(e) = std::fs::create_dir_all(&joined_photo_album_dir) {
                log!(
                    "Warning: UIImageWriteToSavedPhotosAlbum failed to create {:?}: {:?}",
                    joined_photo_album_dir,
                    e
                );
            } else {
                // Scan existing files to pick the next IMG_#### number
                let mut max_index: u32 = 0;
                if let Ok(entries) = std::fs::read_dir(&joined_photo_album_dir) {
                    for entry_res in entries {
                        if let Ok(entry) = entry_res {
                            let name_os = entry.file_name();
                            if let Some(name) = name_os.to_str() {
                                if name.len() >= 8 && name.starts_with("IMG_") {
                                    let num = &name[4..8];
                                    if let Ok(n) = num.parse::<u32>() {
                                        if n > max_index {
                                            max_index = n;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let next_index = max_index + 1;
                let file_name = format!("IMG_{:04}.PNG", next_index);
                let file_path = joined_photo_album_dir.join(file_name);

                match std::fs::write(&file_path, &png_data) {
                    Ok(()) => {
                        log_dbg!(
                            "UIImageWriteToSavedPhotosAlbum: wrote {:?} ({}×{})",
                            file_path,
                            w,
                            h
                        );
                    }
                    Err(e) => {
                        log!(
                            "Warning: UIImageWriteToSavedPhotosAlbum failed to write {:?}: {:?}",
                            file_path,
                            e
                        );
                    }
                }
            }
        } else {
            log!("UIImageWriteToSavedPhotosAlbum: image has no CGImage, skipping save");
        }
    } else {
        log!("UIImageWriteToSavedPhotosAlbum: image == nil, skipping save");
    }

    // Call completion handler
    if completionTarget != nil {
        let _: () = msg_send(
            env,
            (
                completionTarget,
                completionSelector,
                image,
                nil,
                contextInfo,
            ),
        );
    }
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(UIImageWriteToSavedPhotosAlbum(_, _, _, _))];
