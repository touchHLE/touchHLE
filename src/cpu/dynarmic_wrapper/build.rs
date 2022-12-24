use std::path::Path;

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}
fn link_search(path: &Path) {
    println!("cargo:rustc-link-search=native={}", path.to_str().unwrap());
}
fn link_lib(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

// See https://github.com/rust-lang/cc-rs/issues/565
trait CPPVersion {
    fn cpp_version(&mut self, version: &str) -> &mut Self;
}
impl CPPVersion for cc::Build {
    fn cpp_version(&mut self, version: &str) -> &mut Self {
        // TODO: test this actually works on Windows
        if self.get_compiler().is_like_msvc() {
            self.flag(&format!("/std:{}", version))
        } else {
            self.flag(&format!("-std={}", version))
        }
    }
}

fn main() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../../..");

    let dynarmic_out = cmake::build(workspace_root.join("vendor/dynarmic"));
    link_search(&dynarmic_out.join("lib"));
    link_lib("dynarmic");
    link_search(&dynarmic_out.join("build/externals/fmt"));
    link_lib(if cfg!(debug_assertions) {
        "fmtd"
    } else {
        "fmt"
    });
    link_search(&dynarmic_out.join("build/externals/mcl/src"));
    link_lib("mcl");
    link_search(&dynarmic_out.join("build/externals/Zydis"));
    link_lib("Zydis");
    // rerun-if-changed seems to not work if pointed to a directory :(
    //rerun_if_changed(&workspace_root.join("vendor/dynarmic"));

    cc::Build::new()
        .file(package_root.join("lib.cpp"))
        .cpp(true)
        .cpp_version("c++17")
        .include(dynarmic_out.join("include"))
        .compile("dynarmic_wrapper");
    rerun_if_changed(&package_root.join("lib.cpp"));
}
