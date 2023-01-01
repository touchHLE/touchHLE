use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let mut file = File::create(out_dir.join("gl32core.rs")).unwrap();
    Registry::new(Api::Gl, (3, 2), Profile::Core, Fallbacks::None, [])
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();

    let mut file = File::create(out_dir.join("gl21compat.rs")).unwrap();
    Registry::new(
        Api::Gl,
        (2, 1),
        Profile::Compatibility,
        Fallbacks::None,
        ["GL_EXT_framebuffer_object"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    let mut file = File::create(out_dir.join("gles11.rs")).unwrap();
    Registry::new(
        Api::Gles1,
        (1, 1),
        Profile::Core,
        Fallbacks::None,
        ["GL_OES_framebuffer_object", "GL_OES_rgb8_rgba8"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}
