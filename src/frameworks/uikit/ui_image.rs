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
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{ns_data, ns_string, NSInteger};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::fs::GuestPath;
use crate::image::Image;
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, SEL,
};
use crate::Environment;
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

- (id)stretchableImageWithLeftCapWidth:(NSInteger)_leftCapWidth
                          topCapHeight:(NSInteger)_topCapHeight {
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

- (CGFloat)scale {
    // TODO: support other scales, such as @2x
    1.0
}

- (())drawInRect:(CGRect)rect {
    let context = UIGraphicsGetCurrentContext(env);
    let image = env.objc.borrow::<UIImageHostObject>(this).cg_image;
    CGContextDrawImage(env, context, rect, image);
}

- (())drawAtPoint:(CGPoint)point {
    let context = UIGraphicsGetCurrentContext(env);
    if context == nil {
        log!("Warning: [(UIImage*){:?} drawAtPoint:{:?}] is called with nil context, ignoring.", this, point);
        return;
    }
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

// Undocumented class used in NIBs
// TODO: It's not clear _why_ placeholder is needed?
@implementation UIImageNibPlaceholder: UIImage

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    release(env, this);

    // TODO: decode other attributes
    let key_ns_string = get_static_str(env, "UIResourceName");
    let resource_name: id = msg![env; coder decodeObjectForKey:key_ns_string];

    let res = msg_class![env; UIImage imageNamed:resource_name];
    // TODO: It is not clear if we need to additionally retain here?
    retain(env, res)
}

@end

};

fn UIImageWriteToSavedPhotosAlbum(
    env: &mut Environment,
    image: id,
    completionTarget: id,
    completionSelector: SEL,
    contextInfo: MutVoidPtr,
) {
    log_dbg!(
        "UIImageWriteToSavedPhotosAlbum image:{:?} completionTarget:{:?} completionSelector:{:?}",
        image,
        completionTarget,
        completionSelector,
    );

    let cg_image = if image != nil {
        msg![env; image CGImage]
    } else {
        nil
    };

    if cg_image != nil {
        write_to_saved_photos_album_inner(env, cg_image);
    } else {
        log!("UIImageWriteToSavedPhotosAlbum: image has no CGImage, skipping save");
    }

    // Call completion handler
    if completionTarget != nil && !completionSelector.is_null() {
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

/// Helper function to simplify UIImageWriteToSavedPhotosAlbum
/// Allows several failure points to do an early return
fn write_to_saved_photos_album_inner(env: &mut Environment, cg_image: CGImageRef) {
    let (w, h, rgba, stride) = {
        let img = cg_image::borrow_image(&env.objc, cg_image);
        let (w_u32, h_u32) = img.dimensions();
        let stride = w_u32 as usize * 4;
        let rgba = img.pixels().to_vec();
        (w_u32 as i32, h_u32 as i32, rgba, stride as i32)
    };

    let mut png_data: Vec<u8> = Vec::new();
    let ctx_ptr: *mut std::ffi::c_void = (&mut png_data as *mut Vec<u8>).cast();

    let ok = crate::image::write_png(ctx_ptr, w, h, &rgba, stride);

    if ok == 0 {
        log!("Warning: UIImageWriteToSavedPhotosAlbum: stb_image_write failed to encode PNG");
        return;
    }
    let base = crate::paths::user_data_base_path();
    let album_dir = base.join(crate::paths::PHOTO_ALBUM_DIR);

    if let Err(e) = std::fs::create_dir_all(&album_dir) {
        log!(
            "Warning: UIImageWriteToSavedPhotosAlbum failed to create {:?}: {:?}",
            album_dir,
            e
        );
        return;
    }
    // Find next IMG_####.PNG
    let mut max_index: u32 = 0;
    if let Ok(entries) = std::fs::read_dir(&album_dir) {
        for entry_res in entries {
            let Ok(entry) = entry_res else { continue };
            let name_os = entry.file_name();
            let Some(name) = name_os.to_str() else {
                continue;
            };

            // Accept IMG_0001.PNG / IMG_0001.png etc
            if name.starts_with("IMG_") {
                if let Some(dot_idx) = name.rfind('.') {
                    let num_str = &name[4..dot_idx];
                    if let Ok(n) = num_str.parse::<u32>() {
                        max_index = max_index.max(n);
                    }
                }
            }
        }
    }

    let next_index = max_index + 1;
    let file_name = format!("IMG_{:04}.PNG", next_index);
    let file_path = album_dir.join(file_name);

    if let Err(e) = std::fs::write(&file_path, &png_data) {
        log!(
            "Warning: UIImageWriteToSavedPhotosAlbum failed to write {:?}: {:?}",
            file_path,
            e
        );
        return;
    }
    log_dbg!(
        "UIImageWriteToSavedPhotosAlbum: wrote {:?} ({}×{})",
        file_path,
        w,
        h
    );
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(UIImageWriteToSavedPhotosAlbum(_, _, _, _))];
