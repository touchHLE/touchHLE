use std::path::Path;

/*fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}*/
fn link_search(path: &Path) {
    println!("cargo:rustc-link-search=native={}", path.to_str().unwrap());
}
fn link_lib(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

fn main() {
    // TODO: test this on Windows

    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../../..");

    let mut build = cmake::Config::new(workspace_root.join("vendor/openal-soft"));
    build.define("LIBTYPE", "STATIC");
    let openal_soft_out = build.build();

    link_search(&openal_soft_out.join("lib"));
    link_lib("openal");
    // rerun-if-changed seems to not work if pointed to a directory :(
    //rerun_if_changed(&workspace_root.join("vendor/openal-soft"));
}
