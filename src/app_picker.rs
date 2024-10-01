//! App picker GUI.
//!
//! This also includes a license text viewer. The license text viewer is needed
//! on Android, where the command-line way to view license text doesn't exist.

use corosensei::Coroutine;

use crate::bundle::Bundle;
use crate::environment::ThreadBlock;
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextCreateImage,
};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextFillRect, CGContextRelease, CGContextScaleCTM, CGContextSetRGBFillColor,
    CGContextTranslateCTM,
};
use crate::frameworks::core_graphics::cg_image::{self, kCGImageAlphaPremultipliedLast};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_run_loop::run_run_loop_single_iteration;
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_font::{
    UITextAlignmentCenter, UITextAlignmentLeft, UITextAlignmentRight,
};
use crate::frameworks::uikit::ui_graphics::{UIGraphicsPopContext, UIGraphicsPushContext};
use crate::frameworks::uikit::ui_view::ui_control::ui_button::{
    UIButtonTypeCustom, UIButtonTypeRoundedRect,
};
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlStateNormal,
};
use crate::fs::BundleData;
use crate::image::Image;
use crate::mem::Ptr;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject};
use crate::options::Options;
use crate::paths;
use crate::window::DeviceOrientation;
use crate::Environment;
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

struct AppInfo {
    path: PathBuf,
    display_name: String,
    icon: Option<Image>,
    /// `NSString*`
    display_name_ns_string: Option<id>,
    /// `UIImage*`
    icon_ui_image: Option<id>,
}

pub fn app_picker(options: Options) -> Result<(PathBuf, Vec<String>, Environment), String> {
    // BEFOREMERGE: yeah we should probably fix this
    let apps_dir = paths::user_data_base_path().join(paths::APPS_DIR);

    let apps: Result<Vec<AppInfo>, String> = if !apps_dir.is_dir() {
        Err(format!("The {} directory couldn't be found. Check you're running touchHLE from the right directory.", apps_dir.display()))
    } else {
        enumerate_apps(&apps_dir)
            .map_err(|err| {
                format!(
                    "Couldn't get list of apps in the {} directory: {}.",
                    apps_dir.display(),
                    err
                )
            })
            .and_then(|apps| {
                if apps.is_empty() {
                    Err(format!(
                        "No apps were found in the {} directory.",
                        apps_dir.display()
                    ))
                } else {
                    Ok(apps)
                }
            })
    };

    show_app_picker_gui(options, apps)
}

fn enumerate_apps(apps_dir: &Path) -> Result<Vec<AppInfo>, std::io::Error> {
    let mut apps = Vec::new();
    for app in std::fs::read_dir(apps_dir)? {
        let app_path = app?.path();
        if app_path.extension() != Some(OsStr::new("app"))
            && app_path.extension() != Some(OsStr::new("ipa"))
        {
            continue;
        }

        // TODO: avoid loading the whole FS somehow?
        let (bundle, fs) = match BundleData::open_any(&app_path).and_then(|bundle_data| {
            Bundle::new_bundle_and_fs_from_host_path(bundle_data, /* read_only_mode: */ true)
        }) {
            Ok(ok) => ok,
            Err(e) => {
                log!(
                    "Warning: couldn't open app bundle {}: {} (skipping)",
                    app_path.display(),
                    e
                );
                continue;
            }
        };

        // TODO: what if this crashes?
        let display_name = bundle.display_name().to_owned();

        let icon = match bundle.load_icon(&fs) {
            Ok(icon) => Some(icon),
            Err(e) => {
                log!("Warning: couldn't load icon for app bundle {}: {} (displaying placeholder instead)", app_path.display(), e);
                None
            }
        };

        apps.push(AppInfo {
            path: app_path,
            display_name,
            icon,
            display_name_ns_string: None,
            icon_ui_image: None,
        });
    }

    apps.sort_by_key(|app| app.display_name.to_uppercase());

    Ok(apps)
}

#[derive(Default)]
struct AppPickerDelegateHostObject {
    icon_tapped: id,
    copyright_show: bool,
    copyright_hide: bool,
    copyright_prev: bool,
    copyright_next: bool,
    quick_options_show: bool,
    quick_options_hide: bool,
    scale_hack_default: bool,
    scale_hack1: bool,
    scale_hack2: bool,
    scale_hack3: bool,
    scale_hack4: bool,
    orientation_default: bool,
    orientation_landscape_left: bool,
    orientation_landscape_right: bool,
    fullscreen_default: bool,
    fullscreen_on: bool,
}
impl HostObject for AppPickerDelegateHostObject {}

/// Be careful! These classes go in the normal class list, just like everything
/// else, so an app could try to instantiate them. Don't give them special
/// powers that could be exploited!
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_AppPickerDelegate: NSObject

- (())iconTapped:(id)sender {
    // There is no allocWithZone: that creates AppPickerDelegateHostObject, so
    // this downcast effectively acts as an assertion that this class is being
    // used within the app picker, so it can't be abused. :)
    let host_obj = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);
    host_obj.icon_tapped = sender;
}

- (())copyrightInfoShow {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_show = true;
}
- (())copyrightInfoHide {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_hide = true;
}
- (())copyrightInfoPrevPage {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_prev = true;
}
- (())copyrightInfoNextPage {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_next = true;
}

- (())quickOptionsShow {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).quick_options_show = true;
}
- (())quickOptionsHide {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).quick_options_hide = true;
}
- (())scaleHackDefault {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack_default = true;
}
- (())scaleHack1 {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack1 = true;
}
- (())scaleHack2 {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack2 = true;
}
- (())scaleHack3 {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack3 = true;
}
- (())scaleHack4 {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack4 = true;
}
- (())orientationDefault {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_default = true;
}
- (())orientationDefault {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_default = true;
}
- (())orientationLandscapeLeft {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_landscape_left = true;
}
- (())orientationLandscapeRight {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_landscape_right = true;
}
- (())fullscreenDefault {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).fullscreen_default = true;
}
- (())fullscreenOn {
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).fullscreen_on = true;
}

- (())openFileManager {
    // Assert (see above).
    let _ = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);

    match paths::url_for_opening_user_data_dir() {
        Ok(url) => {
            // Our `openURL:` implementation is bypassed because it doesn't
            // allow non-web URLs.
            if let Err(e) = crate::window::open_url(&url) {
                echo!("Couldn't open file manager at {:?}: {}", url, e);
            } else {
                echo!("Opened file manager at {:?}, exiting.", url);
                std::process::exit(0);
            }
        },
        Err(e) => echo!("Couldn't open file manager: {}", e),
    }
}

- (())visitWebsite {
    // Assert (see above).
    let _ = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);

    let url = ns_string::get_static_str(env, "https://touchhle.org/");
    let url: id = msg_class![env; NSURL URLWithString:url];
    let ui_application: id = msg_class![env; UIApplication sharedApplication];
    assert!(msg![env; ui_application openURL:url]);
}

@end

};

fn show_app_picker_gui(
    options: Options,
    apps: Result<Vec<AppInfo>, String>,
) -> Result<(PathBuf, Vec<String>, Environment), String> {
    let icon = {
        let bytes: &[u8] = match super::branding() {
            "" => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/icon.png")),
            "UNOFFICIAL" => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/res/icon-unofficial.png"
            )),
            "PREVIEW" => {
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/icon-preview.png"))
            }
            _ => panic!(),
        };
        let mut image = Image::from_bytes(bytes).unwrap();
        // should match Bundle::load_icon()
        image.round_corners((10.0 / 57.0) * (image.dimensions().0 as f32));
        image
    };

    let mut environment = Environment::new_without_app(options, icon)?;
    let panic_cell = Rc::new(Cell::new(None));
    let mut app_picker_coroutine = Coroutine::new(move |yielder, mut env: Environment| {
        env.panic_cell = panic_cell.clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            env.run_with_yielder(yielder, |env| app_picker_inner(env, apps))
        }));
        if let Err(e) = res {
            let panic_cell = env.panic_cell.clone();
            panic_cell.set(Some(env));
            std::panic::resume_unwind(e);
        }
        res.unwrap().map(|r| (r.0, r.1, env))
    });
    loop {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app_picker_coroutine.resume(environment)
        }));
        environment = match res {
            Ok(ret) => match ret {
                corosensei::CoroutineResult::Yield(env) => env,
                corosensei::CoroutineResult::Return(ret_val) => {
                    return ret_val;
                }
            },
            Err(e) => {
                log_no_panic!("Crash in app picker!");
                // No need to get the environment back - It's local to this
                // function anyways.
                std::panic::resume_unwind(e);
            }
        };
        assert!(environment.threads.len() == 1);
        match environment.threads[0].blocked_by {
            ThreadBlock::NotBlocked => {}
            ThreadBlock::Sleeping(until) => {
                let duration = until.duration_since(Instant::now());
                std::thread::sleep(duration);
            }
            _ => {
                panic!("Unexpected ThreadBlock in app picker!");
            }
        }
        environment.threads[0].blocked_by = ThreadBlock::NotBlocked;
    }
}

fn app_picker_inner(
    env: &mut Environment,
    mut apps: Result<Vec<AppInfo>, String>,
) -> Result<(PathBuf, Vec<String>), String> {
    let mut option_args = Vec::new();
    // Note that objects are generally not released in this code, because they
    // don't need to be: the entire Environment is thrown away at the end.

    // Bypassing UIApplicationMain!
    let ui_application: id = msg_class![env; UIApplication new];
    let delegate = env
        .objc
        .get_known_class("_touchHLE_AppPickerDelegate", &mut env.mem);
    let delegate = env.objc.alloc_object(
        delegate,
        Box::<AppPickerDelegateHostObject>::default(),
        &mut env.mem,
    );
    () = msg![env; ui_application setDelegate:delegate];

    let screen: id = msg_class![env; UIScreen mainScreen];
    let bounds: CGRect = msg![env; screen bounds];

    let window: id = msg_class![env; UIWindow alloc];
    let window: id = msg![env; window initWithFrame:bounds];

    let app_frame: CGRect = msg![env; screen applicationFrame];
    let main_view: id = msg_class![env; UIView alloc];
    let main_view: id = msg![env; main_view initWithFrame:app_frame];
    () = msg![env; window addSubview:main_view];

    // Wallpaper
    let mut found_wallpaper = false;
    let mut have_wallpaper = false;
    for candidate in paths::WALLPAPER_FILES {
        let candidate = paths::user_data_base_path().join(candidate);
        if !candidate.exists() {
            continue;
        }
        found_wallpaper = true;

        let image = match std::fs::read(&candidate) {
            Ok(image) => image,
            Err(e) => {
                log!("Warning: couldn't read {}: {}", candidate.display(), e);
                break;
            }
        };
        let image = match Image::from_bytes(&image) {
            Ok(image) => image,
            Err(e) => {
                log!("Warning: couldn't decode {}: {}", candidate.display(), e);
                break;
            }
        };

        let image = cg_image::from_image(env, image);
        let image: id = msg_class![env; UIImage imageWithCGImage:image];
        let wallpaper: id = msg_class![env; UIImageView alloc];
        let wallpaper: id = msg![env; wallpaper initWithImage:image];
        () = msg![env; wallpaper setFrame:(CGRect {
            origin: CGPoint {
                x: 0.0,
                y: 0.0,
            },
            size: app_frame.size,
        })];
        () = msg![env; wallpaper setAlpha:(0.5 as CGFloat)];
        () = msg![env; main_view addSubview:wallpaper];
        have_wallpaper = true;
        break;
    }
    if !found_wallpaper {
        let CGSize { width, height } = app_frame.size;
        log!(
            "No wallpaper found; filename can be one of: {}; ideal size is {}×{} pixels",
            paths::WALLPAPER_FILES.join(", "),
            width,
            height,
        );
    }

    // Version label
    {
        let label_frame = CGRect {
            origin: CGPoint {
                x: 0.0,
                y: app_frame.size.height - 20.0,
            },
            size: CGSize {
                width: app_frame.size.width - 5.0,
                height: 15.0,
            },
        };
        let label: id = msg_class![env; UILabel alloc];
        let label: id = msg![env; label initWithFrame:label_frame];
        let text = ns_string::from_rust_string(
            env,
            format!(
                "touchHLE {}{}{}",
                super::branding(),
                if super::branding().is_empty() {
                    ""
                } else {
                    " "
                },
                crate::VERSION
            ),
        );
        () = msg![env; label setText:text];
        () = msg![env; label setTextAlignment:UITextAlignmentRight];
        let font_size: CGFloat = 12.0;
        let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
        () = msg![env; label setFont:font];
        let text_color: id = if have_wallpaper {
            msg_class![env; UIColor whiteColor]
        } else {
            msg_class![env; UIColor lightGrayColor]
        };
        () = msg![env; label setTextColor:text_color];
        let bg_color: id = msg_class![env; UIColor clearColor];
        () = msg![env; label setBackgroundColor:bg_color];
        () = msg![env; main_view addSubview:label];
    }

    let brand_color: id = if super::branding() == "UNOFFICIAL" {
        msg_class![env; UIColor redColor]
    } else {
        msg_class![env; UIColor grayColor]
    };

    for i in 1..=7 {
        let label_frame = CGRect {
            origin: CGPoint {
                x: 0.0,
                y: (app_frame.size.height / 8.0) * (i as f32) - 25.0,
            },
            size: CGSize {
                width: app_frame.size.width,
                height: 50.0,
            },
        };
        let label: id = msg_class![env; UILabel alloc];
        let label: id = msg![env; label initWithFrame:label_frame];
        let text = ns_string::from_rust_string(env, super::branding().to_owned());
        () = msg![env; label setText:text];
        () = msg![env; label setTextAlignment:(if i % 2 == 0 { UITextAlignmentLeft } else { UITextAlignmentRight })];
        let font_size: CGFloat = 48.0;
        let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
        () = msg![env; label setFont:font];
        () = msg![env; label setTextColor:brand_color];
        let bg_color: id = msg_class![env; UIColor clearColor];
        () = msg![env; label setBackgroundColor:bg_color];
        () = msg![env; main_view addSubview:label];
    }

    let divider = app_frame.size.height - 100.0;

    let mut icon_grid_stuff = match &mut apps {
        Ok(ref mut apps) => {
            let mut icon_grid_stuff = make_icon_grid(
                env,
                delegate,
                main_view,
                app_frame,
                apps.len(),
                have_wallpaper,
            );
            update_icon_grid(env, &mut icon_grid_stuff, apps, 0);
            Some(icon_grid_stuff)
        }
        Err(e) => {
            let label_frame = CGRect {
                origin: CGPoint { x: 10.0, y: 10.0 },
                size: CGSize {
                    width: app_frame.size.width - 20.0,
                    height: divider - 20.0,
                },
            };
            let label: id = msg_class![env; UILabel alloc];
            let label: id = msg![env; label initWithFrame:label_frame];
            let text = ns_string::from_rust_string(env, e.clone());
            () = msg![env; label setText:text];
            () = msg![env; label setTextAlignment:UITextAlignmentCenter];
            () = msg![env; label setNumberOfLines:0]; // unlimited
            let text_color: id = msg_class![env; UIColor lightGrayColor];
            () = msg![env; label setTextColor:text_color];
            let bg_color: id = msg_class![env; UIColor clearColor];
            () = msg![env; label setBackgroundColor:bg_color];
            () = msg![env; main_view addSubview:label];
            None
        }
    };

    let buttons_row_center = divider + (app_frame.size.height - divider) / 4.0;
    let buttons_row2_center = divider + (app_frame.size.height - divider) / 1.6;
    make_button_row(
        env,
        delegate,
        main_view,
        app_frame.size,
        buttons_row_center,
        &[
            ("File manager", "openFileManager"),
            ("Quick options", "quickOptionsShow"),
        ],
        None,
    );
    make_button_row(
        env,
        delegate,
        main_view,
        app_frame.size,
        buttons_row2_center,
        &[
            ("Copyright info", "copyrightInfoShow"),
            ("touchHLE.org", "visitWebsite"),
        ],
        None,
    );

    let copyright_info_text = crate::licenses::get_text();
    let mut copyright_info_stuff = setup_copyright_info(env, delegate, main_view, app_frame);
    let mut copyright_info_page_idx = 0;

    let quick_options_stuff = setup_quick_options(env, delegate, main_view, app_frame);
    let mut quick_options_scale_hack: Option<NonZeroU32> = None;
    let mut quick_options_fullscreen: Option<()> = None;
    let mut quick_options_orientation: Option<DeviceOrientation> = None;

    fn update_quick_option_buttons(env: &mut Environment, buttons: &[id], selected_idx: usize) {
        for (idx, &button) in buttons.iter().enumerate() {
            let color: id = if idx == selected_idx {
                msg_class![env; UIColor magentaColor]
            } else {
                msg_class![env; UIColor grayColor]
            };
            () = msg![env; button setBackgroundColor:color];
        }
    }
    fn update_scale_hack_buttons(env: &mut Environment, buttons: &[id], value: Option<NonZeroU32>) {
        update_quick_option_buttons(env, buttons, value.map_or(0, |v| (v.get() as usize)));
    }
    fn update_fullscreen_buttons(env: &mut Environment, buttons: &[id], value: Option<()>) {
        update_quick_option_buttons(env, buttons, value.map_or(0, |_| 1));
    }
    fn update_orientation_buttons(
        env: &mut Environment,
        buttons: &[id],
        value: Option<DeviceOrientation>,
    ) {
        update_quick_option_buttons(
            env,
            buttons,
            value.map_or(0, |v| match v {
                DeviceOrientation::LandscapeLeft => 1,
                DeviceOrientation::LandscapeRight => 2,
                _ => panic!(),
            }),
        );
    }
    update_scale_hack_buttons(
        env,
        &quick_options_stuff.scale_hack_buttons,
        quick_options_scale_hack,
    );
    if let Some(ref buttons) = quick_options_stuff.fullscreen_buttons {
        update_fullscreen_buttons(env, buttons, quick_options_fullscreen);
    }
    update_orientation_buttons(
        env,
        &quick_options_stuff.orientation_buttons,
        quick_options_orientation,
    );

    let main_run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    // If an app is picked, this loop returns. If the user quits touchHLE, the
    // process exits.
    let app_path = loop {
        run_run_loop_single_iteration(env, main_run_loop);
        let host_obj = env.objc.borrow_mut::<AppPickerDelegateHostObject>(delegate);
        let icon_tapped = std::mem::take(&mut host_obj.icon_tapped);
        if icon_tapped != nil {
            match icon_grid_stuff.as_ref().unwrap().icon_map.get(&icon_tapped) {
                Some(&TappedIcon::App(app_idx)) => {
                    let app_path = &apps.as_ref().unwrap()[app_idx].path;
                    echo!("Picked: {}", app_path.display());
                    break app_path.clone();
                }
                Some(&TappedIcon::ChangePage(page_idx)) => {
                    update_icon_grid(
                        env,
                        icon_grid_stuff.as_mut().unwrap(),
                        apps.as_mut().unwrap(),
                        page_idx,
                    );
                }
                None => (), // Tapped on a black space
            }
            continue;
        }
        if std::mem::take(&mut host_obj.copyright_show) {
            copyright_info_page_idx = 0;
            change_copyright_page(
                env,
                &mut copyright_info_stuff,
                &copyright_info_text,
                copyright_info_page_idx,
            );
            () = msg![env; (copyright_info_stuff.main_view) setHidden:false];
        } else if std::mem::take(&mut host_obj.copyright_hide) {
            () = msg![env; (copyright_info_stuff.main_view) setHidden:true];
        } else if std::mem::take(&mut host_obj.copyright_prev) && copyright_info_page_idx != 0 {
            copyright_info_page_idx -= 1;
            change_copyright_page(
                env,
                &mut copyright_info_stuff,
                &copyright_info_text,
                copyright_info_page_idx,
            );
        } else if std::mem::take(&mut host_obj.copyright_next)
            && Some(copyright_info_page_idx) != copyright_info_stuff.last_page_idx
        {
            copyright_info_page_idx += 1;
            change_copyright_page(
                env,
                &mut copyright_info_stuff,
                &copyright_info_text,
                copyright_info_page_idx,
            );
        } else if std::mem::take(&mut host_obj.quick_options_show) {
            () = msg![env; (quick_options_stuff.main_view) setHidden:false];
        } else if std::mem::take(&mut host_obj.quick_options_hide) {
            () = msg![env; (quick_options_stuff.main_view) setHidden:true];
        } else if std::mem::take(&mut host_obj.scale_hack_default) {
            quick_options_scale_hack = None;
            update_scale_hack_buttons(
                env,
                &quick_options_stuff.scale_hack_buttons,
                quick_options_scale_hack,
            );
        } else if std::mem::take(&mut host_obj.scale_hack1) {
            quick_options_scale_hack = Some(NonZeroU32::new(1).unwrap());
            update_scale_hack_buttons(
                env,
                &quick_options_stuff.scale_hack_buttons,
                quick_options_scale_hack,
            );
        } else if std::mem::take(&mut host_obj.scale_hack2) {
            quick_options_scale_hack = Some(NonZeroU32::new(2).unwrap());
            update_scale_hack_buttons(
                env,
                &quick_options_stuff.scale_hack_buttons,
                quick_options_scale_hack,
            );
        } else if std::mem::take(&mut host_obj.scale_hack3) {
            quick_options_scale_hack = Some(NonZeroU32::new(3).unwrap());
            update_scale_hack_buttons(
                env,
                &quick_options_stuff.scale_hack_buttons,
                quick_options_scale_hack,
            );
        } else if std::mem::take(&mut host_obj.scale_hack4) {
            quick_options_scale_hack = Some(NonZeroU32::new(4).unwrap());
            update_scale_hack_buttons(
                env,
                &quick_options_stuff.scale_hack_buttons,
                quick_options_scale_hack,
            );
        } else if std::mem::take(&mut host_obj.orientation_default) {
            quick_options_orientation = None;
            update_orientation_buttons(
                env,
                &quick_options_stuff.orientation_buttons,
                quick_options_orientation,
            );
        } else if std::mem::take(&mut host_obj.orientation_landscape_left) {
            quick_options_orientation = Some(DeviceOrientation::LandscapeLeft);
            update_orientation_buttons(
                env,
                &quick_options_stuff.orientation_buttons,
                quick_options_orientation,
            );
        } else if std::mem::take(&mut host_obj.orientation_landscape_right) {
            quick_options_orientation = Some(DeviceOrientation::LandscapeRight);
            update_orientation_buttons(
                env,
                &quick_options_stuff.orientation_buttons,
                quick_options_orientation,
            );
        } else if std::mem::take(&mut host_obj.fullscreen_default) {
            quick_options_fullscreen = None;
            update_fullscreen_buttons(
                env,
                &quick_options_stuff.fullscreen_buttons.unwrap(),
                quick_options_fullscreen,
            );
        } else if std::mem::take(&mut host_obj.fullscreen_on) {
            quick_options_fullscreen = Some(());
            update_fullscreen_buttons(
                env,
                &quick_options_stuff.fullscreen_buttons.unwrap(),
                quick_options_fullscreen,
            );
        }
    };

    // Apply user-specified overrides
    if let Some(scale_hack) = quick_options_scale_hack {
        option_args.push(format!("--scale-hack={}", scale_hack.get()));
    }
    if let Some(orientation) = quick_options_orientation {
        option_args.push(
            match orientation {
                DeviceOrientation::LandscapeLeft => "--landscape-left",
                DeviceOrientation::LandscapeRight => "--landscape-right",
                _ => todo!(),
            }
            .to_string(),
        );
    }
    if let Some(()) = quick_options_fullscreen {
        option_args.push("--fullscreen".to_string());
    }

    // Return the environment so some parts of it can be salvaged.
    Ok((app_path, option_args))
}

const ICON_SIZE: CGSize = CGSize {
    width: 57.0,
    height: 57.0,
};

enum TappedIcon {
    App(usize),
    ChangePage(usize),
}

struct IconGridStuff {
    icon_buttons_and_labels: Vec<(id, id)>,
    placeholder_icon: Option<id>,
    prev_icon: Option<id>,
    next_icon: Option<id>,
    pages: Vec<std::ops::Range<usize>>,
    icon_map: HashMap<id, TappedIcon>,
}

fn make_icon_grid(
    env: &mut Environment,
    delegate: id,
    main_view: id,
    app_frame: CGRect,
    total_app_count: usize,
    have_wallpaper: bool,
) -> IconGridStuff {
    let num_cols = 4;
    let num_cols_f = num_cols as CGFloat;
    let num_rows = 4;
    let label_size = CGSize {
        width: 74.0,
        height: 13.0,
    };
    let icon_gap_x: CGFloat = 19.0;
    let icon_gap_y: CGFloat = 4.0 + label_size.height + 14.0;
    let icon_grid_width = (ICON_SIZE.width * num_cols_f) + icon_gap_x * (num_cols_f - 1.0);
    let icon_grid_origin = CGPoint {
        x: (app_frame.size.width - icon_grid_width) / 2.0,
        y: 12.0,
    };

    let icon_tapped_sel = env.objc.lookup_selector("iconTapped:").unwrap();

    let mut icon_buttons_and_labels = Vec::new();

    for i in 0..(num_cols * num_rows) {
        let col = i % num_cols;
        let row = i / num_cols;

        let icon_frame = CGRect {
            origin: CGPoint {
                x: icon_grid_origin.x + (col as CGFloat) * (ICON_SIZE.width + icon_gap_x),
                y: icon_grid_origin.y + (row as CGFloat) * (ICON_SIZE.height + icon_gap_y),
            },
            size: ICON_SIZE,
        };
        let icon_button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeCustom];
        () = msg![env; icon_button setFrame:icon_frame];
        let image_view: id = msg![env; icon_button imageView];
        let bounds: CGRect = msg![env; icon_button bounds];
        () = msg![env; image_view setFrame:bounds];
        () = msg![env; icon_button addTarget:delegate
                                      action:icon_tapped_sel
                            forControlEvents:UIControlEventTouchUpInside];
        () = msg![env; main_view addSubview:icon_button];

        let label_frame = CGRect {
            origin: CGPoint {
                x: icon_frame.origin.x - (label_size.width - ICON_SIZE.width) / 2.0,
                y: icon_frame.origin.y + ICON_SIZE.height + 4.0,
            },
            size: label_size,
        };
        let label: id = msg_class![env; UILabel alloc];
        let label: id = msg![env; label initWithFrame:label_frame];
        () = msg![env; label setTextAlignment:UITextAlignmentCenter];
        let font_size: CGFloat = label_size.height - 2.0;
        let font: id = if have_wallpaper {
            msg_class![env; UIFont systemFontOfSize:font_size]
        } else {
            msg_class![env; UIFont boldSystemFontOfSize:font_size]
        };
        () = msg![env; label setFont:font];
        let text_color: id = if have_wallpaper {
            msg_class![env; UIColor whiteColor]
        } else {
            msg_class![env; UIColor lightGrayColor]
        };
        () = msg![env; label setTextColor:text_color];
        let bg_color: id = msg_class![env; UIColor clearColor];
        () = msg![env; label setBackgroundColor:bg_color];
        () = msg![env; main_view addSubview:label];

        icon_buttons_and_labels.push((icon_button, label));
    }

    // TODO: Use UIScrollView pagination and UIPageControl once available.
    let mut pages = Vec::new();
    let mut start = 0;
    while start < total_app_count {
        let mut end = start + icon_buttons_and_labels.len();
        if start > 0 {
            end -= 1; // one icon space taken by "previous" button
        }
        if end < total_app_count {
            end -= 1; // one icon space taken by "next" button
        } else {
            end = total_app_count;
        }
        pages.push(start..end);
        start = end;
    }

    IconGridStuff {
        icon_buttons_and_labels,
        placeholder_icon: None,
        prev_icon: None,
        next_icon: None,
        pages,
        icon_map: HashMap::new(),
    }
}

fn make_icon_from_glyph(
    env: &mut Environment,
    glyph: char,
    font_size: CGFloat,
    baseline_offset: CGFloat,
    bg_color: (CGFloat, CGFloat, CGFloat, CGFloat),
) -> id {
    let color_space = CGColorSpaceCreateDeviceRGB(env);
    let context = CGBitmapContextCreate(
        env,
        Ptr::null(),
        ICON_SIZE.width as u32,
        ICON_SIZE.height as u32,
        8,
        4 * (ICON_SIZE.width as u32),
        color_space,
        kCGImageAlphaPremultipliedLast,
    );
    UIGraphicsPushContext(env, context);

    // Compensate for row order inversion
    CGContextTranslateCTM(env, context, 0.0, ICON_SIZE.height);
    CGContextScaleCTM(env, context, 1.0, -1.0);

    let (r, g, b, a) = bg_color;
    CGContextSetRGBFillColor(env, context, r, g, b, a);
    CGContextFillRect(
        env,
        context,
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: ICON_SIZE,
        },
    );

    let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
    let glyph_string: id = ns_string::from_rust_string(env, [glyph].into_iter().collect());
    let glyph_size: CGSize = msg![env; glyph_string sizeWithFont:font];
    CGContextSetRGBFillColor(env, context, 1.0, 1.0, 1.0, 1.0); // white
    let glyph_origin = CGPoint {
        x: ICON_SIZE.width / 2.0 - glyph_size.width / 2.0,
        y: ICON_SIZE.height / 2.0 - glyph_size.height / 2.0 + baseline_offset,
    };
    let _: CGSize = msg![env; glyph_string drawAtPoint:glyph_origin withFont:font];
    release(env, glyph_string);

    UIGraphicsPopContext(env);

    let cg_image = CGBitmapContextCreateImage(env, context);
    // This radius should match the one in src/bundle.rs.
    cg_image::borrow_image_mut(&mut env.objc, cg_image)
        .round_corners((10.0 / 57.0) * ICON_SIZE.width);
    CGContextRelease(env, context);

    let ui_image: id = msg_class![env; UIImage imageWithCGImage:cg_image];
    release(env, cg_image);

    ui_image
}

fn update_icon_grid(
    env: &mut Environment,
    icon_grid_stuff: &mut IconGridStuff,
    apps: &mut [AppInfo],
    page_idx: usize,
) {
    icon_grid_stuff.icon_map.clear();

    let app_idx_range = icon_grid_stuff.pages[page_idx].clone();
    let have_prev_icon = page_idx != 0;
    let have_next_icon = app_idx_range.end != apps.len();

    let mut icon_iter = icon_grid_stuff.icon_buttons_and_labels.iter();

    if have_prev_icon {
        let &(icon_button, label) = icon_iter.next().unwrap();
        let image = *icon_grid_stuff.prev_icon.get_or_insert_with(|| {
            make_icon_from_glyph(env, '←', 50.0, -9.0, (0.25, 0.25, 0.25, 1.0))
        });
        () = msg![env; icon_button setImage:image forState:UIControlStateNormal];
        () = msg![env; label setText:(ns_string::get_static_str(env, ""))];
        icon_grid_stuff
            .icon_map
            .insert(icon_button, TappedIcon::ChangePage(page_idx - 1));
    }

    for app_idx in app_idx_range.clone() {
        let app = &mut apps[app_idx];

        let &(icon_button, label) = icon_iter.next().unwrap();

        if let Some(icon) = app.icon.take() {
            let image = cg_image::from_image(env, icon);
            let image: id = msg_class![env; UIImage imageWithCGImage:image];
            app.icon_ui_image = Some(image);
        }

        let image = app.icon_ui_image.unwrap_or_else(|| {
            *icon_grid_stuff.placeholder_icon.get_or_insert_with(|| {
                make_icon_from_glyph(env, '?', 40.0, 0.0, (0.5, 0.5, 0.5, 1.0))
            })
        });
        () = msg![env; icon_button setImage:image forState:UIControlStateNormal];

        let text = *app
            .display_name_ns_string
            .get_or_insert_with(|| ns_string::from_rust_string(env, app.display_name.clone()));
        () = msg![env; label setText:text];

        icon_grid_stuff
            .icon_map
            .insert(icon_button, TappedIcon::App(app_idx));
    }

    if have_next_icon {
        let &(icon_button, label) = icon_iter.next().unwrap();
        let image = *icon_grid_stuff.next_icon.get_or_insert_with(|| {
            make_icon_from_glyph(env, '→', 50.0, -9.0, (0.25, 0.25, 0.25, 1.0))
        });
        () = msg![env; icon_button setImage:image forState:UIControlStateNormal];
        () = msg![env; label setText:(ns_string::get_static_str(env, ""))];
        icon_grid_stuff
            .icon_map
            .insert(icon_button, TappedIcon::ChangePage(page_idx + 1));
    }

    // There may be remaining spaces might need to be blanked.
    for &(icon_button, label) in icon_iter {
        () = msg![env; icon_button setImage:nil forState:UIControlStateNormal];
        () = msg![env; label setText:(ns_string::get_static_str(env, ""))];
    }
}

fn make_button_row(
    env: &mut Environment,
    delegate: id,
    super_view: id,
    super_view_size: CGSize,
    buttons_row_center: CGFloat,
    buttons: &[(&'static str, &'static str)],
    font_size: Option<CGFloat>,
) -> Vec<id> {
    let margin = 10.0;

    let button_size = CGSize {
        width: (super_view_size.width - margin) / (buttons.len() as CGFloat) - margin,
        height: 30.0,
    };
    let mut button_frame = CGRect {
        origin: CGPoint {
            x: margin,
            y: buttons_row_center - button_size.height / 2.0,
        },
        size: button_size,
    };

    let mut ui_buttons = Vec::new();
    for (title_text, selector) in buttons {
        let button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeRoundedRect];
        let text = ns_string::get_static_str(env, title_text);
        () = msg![env; button setTitle:text forState:UIControlStateNormal];
        () = msg![env; button setFrame:button_frame];
        // FIXME: manually calling layoutSubviews shouldn't be needed?
        () = msg![env; button layoutSubviews];

        if let Some(font_size) = font_size {
            let label: id = msg![env; button titleLabel];
            let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
            () = msg![env; label setFont:font];
        }

        let selector = env.objc.lookup_selector(selector).unwrap();
        () = msg![env; button addTarget:delegate
                                 action:selector
                       forControlEvents:UIControlEventTouchUpInside];
        () = msg![env; super_view addSubview:button];

        button_frame.origin.x += button_size.width + margin;
        ui_buttons.push(button);
    }
    ui_buttons
}

struct CopyrightInfoStuff {
    main_view: id,
    text_frame: CGRect,
    text_label: id,
    font: id,
    pages: Vec<(std::ops::Range<usize>, CGFloat)>,
    last_page_idx: Option<usize>,
    prev_page_button: id,
    next_page_button: id,
}

fn setup_copyright_info(
    env: &mut Environment,
    delegate: id,
    super_view: id,
    app_frame: CGRect,
) -> CopyrightInfoStuff {
    let main_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: app_frame.size,
    };

    let divider = main_frame.size.height - 40.0;

    // Container for all the other stuff

    let main_view: id = msg_class![env; UIView alloc];
    let main_view: id = msg![env; main_view initWithFrame:main_frame];
    // TODO: Isn't white the default?
    let bg_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; main_view setBackgroundColor:bg_color];
    // This main_view is hidden until the copyright info button is tapped.
    () = msg![env; main_view setHidden:true];
    () = msg![env; super_view addSubview:main_view];

    // UILabel that will display part of the copyright text

    let padding = 10.0;
    let text_frame = CGRect {
        origin: CGPoint {
            x: padding,
            y: padding,
        },
        size: CGSize {
            width: app_frame.size.width - padding * 2.0,
            height: divider - padding * 2.0,
        },
    };

    let text_label: id = msg_class![env; UILabel alloc];
    let text_label: id = msg![env; text_label initWithFrame:text_frame];
    () = msg![env; text_label setNumberOfLines:0]; // unlimited
    let text_color: id = msg_class![env; UIColor blackColor];
    () = msg![env; text_label setTextColor:text_color];
    let bg_color: id = msg_class![env; UIColor clearColor];
    () = msg![env; text_label setBackgroundColor:bg_color];
    let font_size: CGFloat = 16.0;
    let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
    () = msg![env; text_label setFont:font];
    () = msg![env; main_view addSubview:text_label];

    // Navigation

    let buttons_row_center = (main_frame.size.height + divider) / 2.0;
    let buttons = make_button_row(
        env,
        delegate,
        main_view,
        main_frame.size,
        buttons_row_center,
        &[
            ("↑", "copyrightInfoPrevPage"),
            ("↓", "copyrightInfoNextPage"),
            ("×", "copyrightInfoHide"),
        ],
        Some(30.0),
    );

    CopyrightInfoStuff {
        main_view,
        text_frame,
        text_label,
        font,
        pages: Vec::new(),
        last_page_idx: None,
        prev_page_button: buttons[0],
        next_page_button: buttons[1],
    }
}

fn change_copyright_page(
    env: &mut Environment,
    copyright_info_stuff: &mut CopyrightInfoStuff,
    copyright_info_text: &str,
    page_idx: usize,
) {
    // TODO: Eventually this should be ripped out and replaced with a scrolling
    // UITextView, once that's implemented.

    let &mut CopyrightInfoStuff {
        text_frame,
        text_label,
        font,
        ref mut pages,
        ref mut last_page_idx,
        prev_page_button,
        next_page_button,
        ..
    } = copyright_info_stuff;

    // Lazily lay out pages of text as needed.

    if page_idx == pages.len() {
        let mut page_start = pages.last().map_or(0, |page| page.0.end);
        while copyright_info_text[page_start..].starts_with([' ', '\n', '\r']) {
            page_start += 1;
        }
        let mut page_height = 0.0;
        let page_end = loop {
            let mut line_start = page_start;
            while line_start < copyright_info_text.len() {
                let is_first_line = line_start == page_start;

                let line_end = if let Some(i) = copyright_info_text[line_start..].find('\n') {
                    line_start + i + 1
                } else {
                    copyright_info_text.len()
                };

                let line = &copyright_info_text[line_start..line_end];

                // Force pagination before headings (in Dynarmic's license text)
                if !is_first_line && line.starts_with("###") {
                    break;
                }

                let line_temp = ns_string::from_rust_string(env, line.to_string());
                let line_size: CGSize = msg![env; line_temp sizeWithFont:font
                                                       constrainedToSize:(text_frame.size)];
                // Avoid accumulation of old line strings.
                release(env, line_temp);

                if page_height + line_size.height > text_frame.size.height {
                    break;
                }

                page_height += line_size.height;
                line_start = line_end;

                // Force pagination after dividers
                if !is_first_line && line.starts_with("---") {
                    break;
                }
            }
            let page_end = line_start;
            assert!(page_start != page_end);

            // Avoid entirely blank pages
            if copyright_info_text[page_start..page_end].trim() == "" {
                page_start = page_end;
            } else {
                break page_end;
            }
        };
        assert!(page_start != page_end);
        pages.push((page_start..page_end, page_height));
        if page_end == copyright_info_text.len() {
            *last_page_idx = Some(page_idx);
        }
    }

    // Actually display the page

    let (page, page_height) = pages[page_idx].clone();
    let page = &copyright_info_text[page];

    let page: id = ns_string::from_rust_string(env, page.to_string());
    () = msg![env; text_label setText:page];
    // Avoid accumulation of old page strings.
    release(env, page);

    // UILabel always vertically centers text. Work around that by resizing it.
    let label_frame = CGRect {
        origin: text_frame.origin,
        size: CGSize {
            width: text_frame.size.width,
            // The page height is slightly off, a little padding is needed.
            height: page_height + 10.0,
        },
    };
    () = msg![env; text_label setFrame:label_frame];

    () = msg![env; prev_page_button setHidden:(page_idx == 0)];
    () = msg![env; next_page_button setHidden:(Some(page_idx) == *last_page_idx)];
}

struct QuickOptionsStuff {
    main_view: id,
    scale_hack_buttons: [id; 5],
    orientation_buttons: [id; 3],
    fullscreen_buttons: Option<[id; 2]>,
}

fn setup_quick_options(
    env: &mut Environment,
    delegate: id,
    super_view: id,
    app_frame: CGRect,
) -> QuickOptionsStuff {
    // UIView*
    let main_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: app_frame.size,
    };

    // Container for all the other stuff

    let main_view: id = msg_class![env; UIView alloc];
    let main_view: id = msg![env; main_view initWithFrame:main_frame];
    // TODO: Isn't white the default?
    let bg_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; main_view setBackgroundColor:bg_color];
    // This main_view is hidden until the copyright info button is tapped.
    () = msg![env; main_view setHidden:true];
    () = msg![env; super_view addSubview:main_view];

    let divider = 40.0;

    // Close button
    {
        let button_frame = CGRect {
            origin: CGPoint {
                x: main_frame.size.width - 30.0,
                y: 10.0,
            },
            size: CGSize {
                width: 20.0,
                height: 20.0,
            },
        };

        let button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeRoundedRect];
        let text = ns_string::get_static_str(env, "×");
        () = msg![env; button setTitle:text forState:UIControlStateNormal];
        () = msg![env; button setFrame:button_frame];
        // FIXME: manually calling layoutSubviews shouldn't be needed?
        () = msg![env; button layoutSubviews];

        let label: id = msg![env; button titleLabel];
        let font: id = msg_class![env; UIFont systemFontOfSize:(30.0 as CGFloat)];
        () = msg![env; label setFont:font];

        let selector = env.objc.lookup_selector("quickOptionsHide").unwrap();
        () = msg![env; button addTarget:delegate
                                 action:selector
                       forControlEvents:UIControlEventTouchUpInside];
        () = msg![env; main_view addSubview:button];
    }

    enum RowKind {
        Label(&'static str),
        Buttons(&'static [(&'static str, &'static str)]),
    }
    let rows = [
        RowKind::Label("Scale hack"),
        RowKind::Buttons(&[
            ("Default", "scaleHackDefault"),
            ("Off", "scaleHack1"),
            ("2×", "scaleHack2"),
            ("3×", "scaleHack3"),
            ("4×", "scaleHack4"),
        ]),
        RowKind::Label("Orientation"),
        RowKind::Buttons(&[
            ("Default", "orientationDefault"),
            ("←", "orientationLandscapeLeft"),
            ("→", "orientationLandscapeRight"),
        ]),
        // ---- (divider for stuff skipped below)
        RowKind::Label("Fullscreen"),
        RowKind::Buttons(&[("Default", "fullscreenDefault"), ("On", "fullscreenOn")]),
    ];
    let rows_len_full = rows.len();
    let rows = if crate::window::Window::rotatable_fullscreen() {
        // Fullscreen option doesn't make sense on always-fullscreen platforms
        &rows[..rows.len() - 2]
    } else {
        &rows[..]
    };

    let mut button_rows = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let row_center = divider
            + ((1 + i) as CGFloat)
                * ((main_frame.size.height - divider) / ((rows_len_full + 1) as CGFloat));

        match *row {
            RowKind::Label(text) => {
                let frame = CGRect {
                    origin: CGPoint {
                        x: 0.0,
                        y: row_center - 30.0 / 2.0,
                    },
                    size: CGSize {
                        width: main_frame.size.width,
                        height: 30.0,
                    },
                };

                let label: id = msg_class![env; UILabel alloc];
                let label: id = msg![env; label initWithFrame:frame];
                let text = ns_string::get_static_str(env, text);
                () = msg![env; label setText:text];
                () = msg![env; label setTextAlignment:UITextAlignmentCenter];
                () = msg![env; main_view addSubview:label];
            }
            RowKind::Buttons(buttons) => {
                button_rows.push(make_button_row(
                    env,
                    delegate,
                    main_view,
                    main_frame.size,
                    row_center,
                    buttons,
                    /* font_size: */ None,
                ));
            }
        }
    }

    QuickOptionsStuff {
        main_view,
        scale_hack_buttons: button_rows[0][..].try_into().unwrap(),
        orientation_buttons: button_rows[1][..].try_into().unwrap(),
        fullscreen_buttons: button_rows.get(2).map(|r| r[..].try_into().unwrap()),
    }
}
