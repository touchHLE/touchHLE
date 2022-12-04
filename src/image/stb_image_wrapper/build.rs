use std::path::Path;

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}

fn main() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../../..");

    cc::Build::new()
        .file(package_root.join("lib.c"))
        .compile("stb_image_wrapper");
    rerun_if_changed(&package_root.join("lib.c"));
    rerun_if_changed(&workspace_root.join("vendor/stb/stb_image.h"));
}
