/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::HostDylib;
use crate::gles::gles11_raw;
use crate::objc::{id, msg_class, nil, objc_classes, ClassExports, HostObject};

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/GLKit.framework/GLKit",
    aliases: &[],
    function_exports: &[],
    class_exports: &[&CLASSES],
    constant_exports: &[],
};

struct GLKTextureInfoHostObject {
    name: u32,
    target: u32,
    width: u32,
    height: u32,
}
impl HostObject for GLKTextureInfoHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GLKTextureLoader: NSObject

+ (id)textureWithContentsOfFile:(id)path
                        options:(id)_options
                          error:(id)_outError {
    let path_str = crate::frameworks::foundation::ns_string::to_rust_string(env, path);
    log_dbg!("[GLKTextureLoader textureWithContentsOfFile:{:?} ...]", path_str);

    let bytes = match std::fs::read(path_str.as_ref()) {
        Ok(bytes) => bytes,
        Err(e) => {
            log!("Failed to read texture from file {:?}: {}", path_str, e);
            return nil;
        }
    };

    let image = match crate::image::Image::from_bytes(&bytes) {
        Ok(img) => img,
        Err(e) => {
            log!("Failed to parse image for texture {:?}: {}", path_str, e);
            return nil;
        }
    };

    let (width, height) = image.dimensions();

    let mut texture = 0;
    {
        let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");
        let mut gles = crate::frameworks::opengles::sync_context(&mut env.framework_state.opengles, &mut env.objc, window, env.current_thread);

        unsafe {
            gles.GenTextures(1, &mut texture);
            gles.BindTexture(gles11_raw::TEXTURE_2D, texture);

            gles.TexImage2D(
                gles11_raw::TEXTURE_2D,
                0,
                gles11_raw::RGBA as _,
                width as _,
                height as _,
                0,
                gles11_raw::RGBA,
                gles11_raw::UNSIGNED_BYTE,
                image.pixels().as_ptr() as *const _,
            );

            gles.TexParameteri(
                gles11_raw::TEXTURE_2D,
                gles11_raw::TEXTURE_MIN_FILTER,
                gles11_raw::LINEAR as _,
            );
            gles.TexParameteri(
                gles11_raw::TEXTURE_2D,
                gles11_raw::TEXTURE_MAG_FILTER,
                gles11_raw::LINEAR as _,
            );
        }
    }

    let info: id = msg_class![env; GLKTextureInfo alloc];
    let host_obj = env.objc.borrow_mut::<GLKTextureInfoHostObject>(info);
    host_obj.name = texture;
    host_obj.target = gles11_raw::TEXTURE_2D;
    host_obj.width = width;
    host_obj.height = height;

    info
}

@end

@implementation GLKTextureInfo: NSObject

+ (id)alloc {
    let host_object = Box::new(GLKTextureInfoHostObject {
        name: 0,
        target: 0,
        width: 0,
        height: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (u32)name {
    env.objc.borrow::<GLKTextureInfoHostObject>(this).name
}

- (u32)target {
    env.objc.borrow::<GLKTextureInfoHostObject>(this).target
}

- (u32)width {
    env.objc.borrow::<GLKTextureInfoHostObject>(this).width
}

- (u32)height {
    env.objc.borrow::<GLKTextureInfoHostObject>(this).height
}

@end

};
