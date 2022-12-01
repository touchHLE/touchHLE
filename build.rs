use std::path::Path;

fn main() {
    let root = Path::new(file!()).parent().unwrap();

    cc::Build::new()
        .file(root.join("src/image/stb_image_wrapper.c"))
        .compile("stb_image_wrapper");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src/image/stb_image_wrapper.c").to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("vendor/stb").to_str().unwrap()
    );
}
