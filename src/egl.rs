/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal EGL loader for the bundled ANGLE (Metal backend) implementation.
//!
//! ANGLE's `libEGL.dylib` + `libGLESv2.dylib` are loaded dynamically so that
//! the emulator starts (with a clear error) even if the dylibs are missing.
//! All OpenGL ES contexts are created on a single EGL display/surface pair
//! backed by the window's `CAMetalLayer` (obtained via SDL2's Metal API), and
//! they all share one share group (see [crate::window]).

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub type EGLDisplay = *mut c_void;
pub type EGLConfig = *mut c_void;
pub type EGLContext = *mut c_void;
pub type EGLSurface = *mut c_void;
pub type EGLNativeWindowType = *mut c_void;
pub type EGLint = i32;
pub type EGLBoolean = u32;
pub type EGLenum = u32;

pub const EGL_DEFAULT_DISPLAY: *mut c_void = std::ptr::null_mut();
pub const EGL_NO_CONTEXT: EGLContext = std::ptr::null_mut();
pub const EGL_NO_SURFACE: EGLSurface = std::ptr::null_mut();
pub const EGL_FALSE: EGLBoolean = 0;

pub const EGL_NONE: EGLint = 0x3038;
pub const EGL_SURFACE_TYPE: EGLint = 0x3033;
pub const EGL_WINDOW_BIT: EGLint = 0x0004;
pub const EGL_RENDERABLE_TYPE: EGLint = 0x3040;
pub const EGL_OPENGL_ES_BIT: EGLint = 0x0001;
pub const EGL_OPENGL_ES2_BIT: EGLint = 0x0004;
pub const EGL_CONTEXT_CLIENT_VERSION: EGLint = 0x3098;
pub const EGL_OPENGL_ES_API: EGLenum = 0x30A0;
// Surface-size queries, kept for potential resize handling.
#[allow(dead_code)]
pub const EGL_WIDTH: EGLint = 0x3057;
#[allow(dead_code)]
pub const EGL_HEIGHT: EGLint = 0x3056;

/// ANGLE-specific platform extension constants.
pub const EGL_PLATFORM_ANGLE_ANGLE: EGLenum = 0x3202;
pub const EGL_PLATFORM_ANGLE_TYPE_ANGLE: EGLint = 0x3203;
pub const EGL_PLATFORM_ANGLE_TYPE_METAL_ANGLE: EGLint = 0x3489;

type FnGetDisplay = unsafe extern "C" fn(*mut c_void) -> EGLDisplay;
type FnGetPlatformDisplayEXT =
    unsafe extern "C" fn(EGLenum, *mut c_void, *const EGLint) -> EGLDisplay;
type FnInitialize = unsafe extern "C" fn(EGLDisplay, *mut EGLint, *mut EGLint) -> EGLBoolean;
type FnBindAPI = unsafe extern "C" fn(EGLenum) -> EGLBoolean;
type FnChooseConfig =
    unsafe extern "C" fn(EGLDisplay, *const EGLint, *mut EGLConfig, EGLint, *mut EGLint) -> EGLBoolean;
type FnCreateWindowSurface =
    unsafe extern "C" fn(EGLDisplay, EGLConfig, EGLNativeWindowType, *const EGLint) -> EGLSurface;
type FnCreateContext =
    unsafe extern "C" fn(EGLDisplay, EGLConfig, EGLContext, *const EGLint) -> EGLContext;
type FnDestroyContext = unsafe extern "C" fn(EGLDisplay, EGLContext) -> EGLBoolean;
type FnDestroySurface = unsafe extern "C" fn(EGLDisplay, EGLSurface) -> EGLBoolean;
type FnMakeCurrent =
    unsafe extern "C" fn(EGLDisplay, EGLSurface, EGLSurface, EGLContext) -> EGLBoolean;
type FnSwapBuffers = unsafe extern "C" fn(EGLDisplay, EGLSurface) -> EGLBoolean;
type FnGetCurrentContext = unsafe extern "C" fn() -> EGLContext;
type FnGetError = unsafe extern "C" fn() -> EGLint;
type FnGetProcAddress = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnQuerySurface =
    unsafe extern "C" fn(EGLDisplay, EGLSurface, EGLint, *mut EGLint) -> EGLBoolean;
type FnQueryString = unsafe extern "C" fn(EGLDisplay, EGLint) -> *const c_char;

/// Function pointers loaded from ANGLE's `libEGL.dylib`, plus the dylib
/// handles needed for `dlsym` fallback when resolving GL entry points.
pub struct EglLib {
    _egl_handle: *mut c_void,
    glesv2_handle: *mut c_void,
    eglGetDisplay: FnGetDisplay,
    eglGetPlatformDisplayEXT: Option<FnGetPlatformDisplayEXT>,
    eglInitialize: FnInitialize,
    eglBindAPI: FnBindAPI,
    eglChooseConfig: FnChooseConfig,
    eglCreateWindowSurface: FnCreateWindowSurface,
    eglCreateContext: FnCreateContext,
    eglDestroyContext: FnDestroyContext,
    eglDestroySurface: FnDestroySurface,
    eglMakeCurrent: FnMakeCurrent,
    eglSwapBuffers: FnSwapBuffers,
    eglGetCurrentContext: FnGetCurrentContext,
    eglGetError: FnGetError,
    eglGetProcAddress: FnGetProcAddress,
    eglQuerySurface: FnQuerySurface,
    eglQueryString: FnQueryString,
}
unsafe impl Send for EglLib {}
unsafe impl Sync for EglLib {}

static EGL: LazyLock<EglLib> = LazyLock::new(EglLib::load);

/// Get the global [EglLib], loading ANGLE on first use.
pub fn global() -> &'static EglLib {
    &EGL
}

/// Directory containing ANGLE's dylibs (`libEGL.dylib`, `libGLESv2.dylib`).
///
/// Lookup order:
/// 1. `PEAR_ANGLE_LIB_DIR` environment variable (explicit override).
/// 2. `touchHLE_dylibs/angle` in the app bundle / resources dir / CWD
///    (release layout).
/// 3. `vendor/angle/out/MacMetal` in the CWD (development layout).
fn angle_lib_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PEAR_ANGLE_LIB_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Ok(base) = sdl2::filesystem::base_path() {
        bases.push(PathBuf::from(base));
    }
    if let Some(resources) = crate::paths::get_macos_bundled_resources_path() {
        bases.push(resources);
    }
    bases.push(PathBuf::from("."));
    for base in &bases {
        let candidate = base.join(crate::paths::DYLIBS_DIR).join("angle");
        if candidate.join("libEGL.dylib").exists() {
            return candidate;
        }
    }
    let dev_fallback = PathBuf::from("vendor/angle/out/MacMetal");
    if dev_fallback.join("libEGL.dylib").exists() {
        return dev_fallback;
    }
    panic!(
        "Couldn't find ANGLE's libEGL.dylib. Set PEAR_ANGLE_LIB_DIR or bundle \
         the dylibs under {}/angle.",
        crate::paths::DYLIBS_DIR
    );
}

fn dylopen(path: &Path) -> *mut c_void {
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
    if handle.is_null() {
        let err = unsafe { std::ffi::CStr::from_ptr(libc::dlerror()) };
        panic!("Couldn't load {}: {}", path.display(), err.to_string_lossy());
    }
    handle
}

fn dlsym<T>(handle: *mut c_void, name: &str) -> T {
    let c_name = CString::new(name).unwrap();
    let sym = unsafe { libc::dlsym(handle, c_name.as_ptr()) };
    assert!(!sym.is_null(), "Missing symbol {} in ANGLE dylib", name);
    unsafe { std::mem::transmute_copy(&sym) }
}

fn dlsym_opt<T>(handle: *mut c_void, name: &str) -> Option<T> {
    let c_name = CString::new(name).unwrap();
    let sym = unsafe { libc::dlsym(handle, c_name.as_ptr()) };
    if sym.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy(&sym) })
    }
}

impl EglLib {
    fn load() -> Self {
        let dir = angle_lib_dir();
        log!("Loading ANGLE (Metal) from {:?}.", dir);
        // libGLESv2 first: libEGL depends on it and dyld resolves it as a
        // sibling via @loader_path.
        let glesv2_handle = dylopen(&dir.join("libGLESv2.dylib"));
        let egl_handle = dylopen(&dir.join("libEGL.dylib"));
        EglLib {
            eglGetDisplay: dlsym(egl_handle, "eglGetDisplay"),
            eglGetPlatformDisplayEXT: dlsym_opt(egl_handle, "eglGetPlatformDisplayEXT"),
            eglInitialize: dlsym(egl_handle, "eglInitialize"),
            eglBindAPI: dlsym(egl_handle, "eglBindAPI"),
            eglChooseConfig: dlsym(egl_handle, "eglChooseConfig"),
            eglCreateWindowSurface: dlsym(egl_handle, "eglCreateWindowSurface"),
            eglCreateContext: dlsym(egl_handle, "eglCreateContext"),
            eglDestroyContext: dlsym(egl_handle, "eglDestroyContext"),
            eglDestroySurface: dlsym(egl_handle, "eglDestroySurface"),
            eglMakeCurrent: dlsym(egl_handle, "eglMakeCurrent"),
            eglSwapBuffers: dlsym(egl_handle, "eglSwapBuffers"),
            eglGetCurrentContext: dlsym(egl_handle, "eglGetCurrentContext"),
            eglGetError: dlsym(egl_handle, "eglGetError"),
            eglGetProcAddress: dlsym(egl_handle, "eglGetProcAddress"),
            eglQuerySurface: dlsym(egl_handle, "eglQuerySurface"),
            eglQueryString: dlsym(egl_handle, "eglQueryString"),
            _egl_handle: egl_handle,
            glesv2_handle,
        }
    }

    fn check<T>(&self, what: &str) -> Result<T, String> {
        let err = unsafe { (self.eglGetError)() };
        if err == 0 {
            return Err(format!("{} failed (no EGL error set)", what));
        }
        Err(format!("{} failed with EGL error {:#x}", what, err))
    }

    /// Initialize an EGL display on ANGLE's Metal backend and pick a config
    /// usable for both OpenGL ES 1.x and 2.0 contexts.
    pub fn initialize(&self) -> Result<(EGLDisplay, EGLConfig), String> {
        let display = unsafe {
            match self.eglGetPlatformDisplayEXT {
                Some(get_platform_display) => {
                    let attribs = [
                        EGL_PLATFORM_ANGLE_TYPE_ANGLE,
                        EGL_PLATFORM_ANGLE_TYPE_METAL_ANGLE,
                        EGL_NONE,
                    ];
                    get_platform_display(
                        EGL_PLATFORM_ANGLE_ANGLE,
                        EGL_DEFAULT_DISPLAY,
                        attribs.as_ptr(),
                    )
                }
                None => (self.eglGetDisplay)(EGL_DEFAULT_DISPLAY),
            }
        };
        if display.is_null() {
            return self.check("eglGetPlatformDisplayEXT(Metal)");
        }

        let (mut major, mut minor) = (0, 0);
        let ok = unsafe { (self.eglInitialize)(display, &mut major, &mut minor) };
        if ok == EGL_FALSE {
            return self.check("eglInitialize");
        }
        log!("EGL initialized: version {}.{}.", major, minor);
        let version = unsafe { (self.eglQueryString)(display, 0x3054 /* EGL_VERSION */) };
        let client_apis = unsafe { (self.eglQueryString)(display, 0x308D /* EGL_CLIENT_APIS */) };
        log!(
            "EGL vendor string: {}.",
            unsafe { CStr::from_ptr((self.eglQueryString)(display, 0x3053 /* EGL_VENDOR */)) }
                .to_string_lossy()
        );
        log!(
            "EGL version: {}; client APIs: {}.",
            if version.is_null() {
                "<unknown>".to_string()
            } else {
                unsafe { CStr::from_ptr(version) }.to_string_lossy().into_owned()
            },
            if client_apis.is_null() {
                "<unknown>".to_string()
            } else {
                unsafe { CStr::from_ptr(client_apis) }.to_string_lossy().into_owned()
            }
        );

        let ok = unsafe { (self.eglBindAPI)(EGL_OPENGL_ES_API) };
        if ok == EGL_FALSE {
            return self.check("eglBindAPI(EGL_OPENGL_ES_API)");
        }

        let config_attribs = [
            EGL_SURFACE_TYPE,
            EGL_WINDOW_BIT,
            EGL_RENDERABLE_TYPE,
            EGL_OPENGL_ES_BIT | EGL_OPENGL_ES2_BIT,
            EGL_NONE,
        ];
        let mut config: EGLConfig = std::ptr::null_mut();
        let mut num_configs = 0;
        let ok = unsafe {
            (self.eglChooseConfig)(display, config_attribs.as_ptr(), &mut config, 1, &mut num_configs)
        };
        if ok == EGL_FALSE || num_configs == 0 || config.is_null() {
            return self.check("eglChooseConfig");
        }
        Ok((display, config))
    }

    pub unsafe fn create_window_surface(
        &self,
        display: EGLDisplay,
        config: EGLConfig,
        native_window: EGLNativeWindowType,
    ) -> Result<EGLSurface, String> {
        let surface =
            (self.eglCreateWindowSurface)(display, config, native_window, std::ptr::null());
        if surface.is_null() {
            return self.check("eglCreateWindowSurface");
        }
        Ok(surface)
    }

    /// Create a context for the given client version (1 or 2), sharing with
    /// `share` (pass [EGL_NO_CONTEXT] for the first context). All contexts
    /// created this way live in one share group, so GL objects (framebuffer
    /// attachments etc.) can cross between them.
    pub unsafe fn create_context(
        &self,
        display: EGLDisplay,
        config: EGLConfig,
        share: EGLContext,
        client_version: u32,
    ) -> Result<EGLContext, String> {
        let attribs = [EGL_CONTEXT_CLIENT_VERSION, client_version as EGLint, EGL_NONE];
        let ctx = (self.eglCreateContext)(display, config, share, attribs.as_ptr());
        if ctx.is_null() {
            return self.check(&format!(
                "eglCreateContext(client version {})",
                client_version
            ));
        }
        Ok(ctx)
    }

    pub unsafe fn destroy_context(&self, display: EGLDisplay, ctx: EGLContext) {
        if !ctx.is_null() {
            (self.eglDestroyContext)(display, ctx);
        }
    }

    pub unsafe fn destroy_surface(&self, display: EGLDisplay, surface: EGLSurface) {
        if !surface.is_null() {
            (self.eglDestroySurface)(display, surface);
        }
    }

    pub unsafe fn make_current(
        &self,
        display: EGLDisplay,
        surface: EGLSurface,
        ctx: EGLContext,
    ) -> Result<(), String> {
        let ok = (self.eglMakeCurrent)(display, surface, surface, ctx);
        if ok == EGL_FALSE {
            return self.check("eglMakeCurrent");
        }
        Ok(())
    }

    pub unsafe fn swap_buffers(&self, display: EGLDisplay, surface: EGLSurface) {
        (self.eglSwapBuffers)(display, surface);
    }

    pub fn current_context(&self) -> EGLContext {
        unsafe { (self.eglGetCurrentContext)() }
    }

    /// Query a surface attribute (e.g. [EGL_WIDTH], [EGL_HEIGHT]). Kept for
    /// potential resize handling.
    #[allow(dead_code)]
    pub unsafe fn query_surface(
        &self,
        display: EGLDisplay,
        surface: EGLSurface,
        attribute: EGLint,
    ) -> EGLint {
        let mut value = 0;
        (self.eglQuerySurface)(display, surface, attribute, &mut value);
        value
    }

    /// Resolve a GL entry point, trying `eglGetProcAddress` first and then
    /// direct `dlsym` on `libGLESv2.dylib`.
    pub fn get_proc_address(&self, name: &str) -> *mut c_void {
        let c_name = CString::new(name).unwrap();
        let sym = unsafe { (self.eglGetProcAddress)(c_name.as_ptr()) };
        if !sym.is_null() {
            return sym;
        }
        unsafe { libc::dlsym(self.glesv2_handle, c_name.as_ptr()) }
    }
}
