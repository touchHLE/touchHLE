//! App picker GUI.

use crate::bundle::Bundle;
use crate::frameworks::core_graphics::{cg_image, CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_run_loop::run_run_loop_single_iteration;
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::ui_button::{
    UIButtonTypeCustom, UIButtonTypeRoundedRect,
};
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlStateNormal,
};
use crate::fs::BundleData;
use crate::image::Image;
use crate::objc::{id, msg, msg_class, nil, objc_classes, ClassExports, HostObject};
use crate::options::Options;
use crate::paths;
use crate::Environment;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

struct AppInfo {
    path: PathBuf,
    display_name: String,
    icon: Option<Image>,
}

pub fn app_picker(options: Options) -> Result<(PathBuf, Environment), String> {
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

        // TODO: avoid loading the whole FS
        let (bundle, fs) = match BundleData::open_any(&app_path)
            .and_then(Bundle::new_bundle_and_fs_from_host_path)
        {
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
        });
    }
    Ok(apps)
}

#[derive(Default)]
struct AppPickerDelegateHostObject {
    app_tapped: id,
}
impl HostObject for AppPickerDelegateHostObject {}

/// Be careful! These classes go in the normal class list, just like everything
/// else, so an app could try to instantiate them. Don't give them special
/// powers that could be exploited!
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_AppPickerDelegate

- (())appTapped:(id)sender {
    // There is no allocWithZone: that creates AppPickerDelegateHostObject, so
    // this downcast effectively acts as an assertion that this class is being
    // used within the app picker, so it can't be abused. :)
    let host_obj = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);
    host_obj.app_tapped = sender;
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
) -> Result<(PathBuf, Environment), String> {
    let mut environment = Environment::new_without_app(options)?;
    let env = &mut environment;

    // Note that no objects are ever released in this code, because they don't
    // need to be: the entire Environment is thrown away at the end.

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

    let divider = app_frame.size.height - 100.0;

    let mut app_map = HashMap::new();

    match apps {
        Ok(apps) => make_icon_grid(env, delegate, main_view, app_frame, apps, &mut app_map),
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
            let text = ns_string::from_rust_string(env, e);
            () = msg![env; label setText:text];
            () = msg![env; label setTextAlignment:UITextAlignmentCenter];
            () = msg![env; label setNumberOfLines:0]; // unlimited
            let text_color: id = msg_class![env; UIColor lightGrayColor];
            () = msg![env; label setTextColor:text_color];
            let bg_color: id = msg_class![env; UIColor clearColor];
            () = msg![env; label setBackgroundColor:bg_color];
            () = msg![env; main_view addSubview:label];
        }
    }

    make_button_row(
        env,
        delegate,
        main_view,
        app_frame,
        divider,
        &[("Visit touchHLE.org", "visitWebsite")],
    );

    let main_run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    // If an app is picked, this loop returns. If the user quits touchHLE, the
    // process exits.
    loop {
        run_run_loop_single_iteration(env, main_run_loop);
        let host_obj = env.objc.borrow_mut::<AppPickerDelegateHostObject>(delegate);
        if host_obj.app_tapped != nil {
            let app_path = app_map.remove(&host_obj.app_tapped).unwrap();
            echo!("Picked: {}", app_path.display());
            // Return the environment so some parts of it can be salvaged.
            return Ok((app_path, environment));
        }
    }
}

fn make_icon_grid(
    env: &mut Environment,
    delegate: id,
    main_view: id,
    app_frame: CGRect,
    apps: Vec<AppInfo>,
    app_map: &mut HashMap<id, PathBuf>,
) {
    let num_cols = 4;
    let num_cols_f = num_cols as CGFloat;
    let num_rows = 4;
    let label_size = CGSize {
        width: 74.0,
        height: 13.0,
    };
    let icon_size = CGSize {
        width: 57.0,
        height: 57.0,
    };
    let icon_gap_x: CGFloat = 19.0;
    let icon_gap_y: CGFloat = 4.0 + label_size.height + 14.0;
    let icon_grid_width = (icon_size.width * num_cols_f) + icon_gap_x * (num_cols_f - 1.0);
    let icon_grid_origin = CGPoint {
        x: (app_frame.size.width - icon_grid_width) / 2.0,
        y: 12.0,
    };

    let app_tapped_sel = env.objc.lookup_selector("appTapped:").unwrap();

    for (i, app) in apps.into_iter().enumerate() {
        if i >= num_cols * num_rows {
            // TODO: add pagination in order to remove app count limit
            log!(
                "Warning: too many apps, only showing the first {}.",
                num_cols * num_rows
            );
            break;
        }
        let col = i % num_cols;
        let row = i / num_cols;

        let icon_frame = CGRect {
            origin: CGPoint {
                x: icon_grid_origin.x + (col as CGFloat) * (icon_size.width + icon_gap_x),
                y: icon_grid_origin.y + (row as CGFloat) * (icon_size.height + icon_gap_y),
            },
            size: icon_size,
        };
        let icon_button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeCustom];
        () = msg![env; icon_button setFrame:icon_frame];
        if let Some(icon) = app.icon {
            let image = cg_image::from_image(env, icon);
            let image: id = msg_class![env; UIImage imageWithCGImage:image];
            () = msg![env; icon_button setImage:image forState:UIControlStateNormal];
            let image_view: id = msg![env; icon_button imageView];
            let bounds: CGRect = msg![env; icon_button bounds];
            () = msg![env; image_view setFrame:bounds];
        } else {
            let text = ns_string::get_static_str(env, "?");
            () = msg![env; icon_button setTitle:text forState:UIControlStateNormal];
            let color: id = msg_class![env; UIColor whiteColor];
            () = msg![env; icon_button setTitleColor:color forState:UIControlStateNormal];
            let bg_color: id = msg_class![env; UIColor grayColor];
            () = msg![env; icon_button setBackgroundColor:bg_color];
            let label: id = msg![env; icon_button titleLabel];
            () = msg![env; label setTextAlignment:UITextAlignmentCenter];
            let font_size: CGFloat = 40.0;
            let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
            () = msg![env; label setFont:font];
            // FIXME: manually calling layoutSubviews shouldn't be needed
            () = msg![env; icon_button layoutSubviews];
        }
        () = msg![env; icon_button addTarget:delegate
                                      action:app_tapped_sel
                            forControlEvents:UIControlEventTouchUpInside];
        () = msg![env; main_view addSubview:icon_button];

        let label_frame = CGRect {
            origin: CGPoint {
                x: icon_frame.origin.x - (label_size.width - icon_size.width) / 2.0,
                y: icon_frame.origin.y + icon_size.height + 4.0,
            },
            size: label_size,
        };
        let label: id = msg_class![env; UILabel alloc];
        let label: id = msg![env; label initWithFrame:label_frame];
        let text = ns_string::from_rust_string(env, app.display_name);
        () = msg![env; label setText:text];
        () = msg![env; label setTextAlignment:UITextAlignmentCenter];
        let font_size: CGFloat = label_size.height - 2.0;
        let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
        () = msg![env; label setFont:font];
        let text_color: id = msg_class![env; UIColor lightGrayColor];
        () = msg![env; label setTextColor:text_color];
        let bg_color: id = msg_class![env; UIColor clearColor];
        () = msg![env; label setBackgroundColor:bg_color];
        () = msg![env; main_view addSubview:label];

        app_map.insert(icon_button, app.path);
    }
}

fn make_button_row(
    env: &mut Environment,
    delegate: id,
    main_view: id,
    app_frame: CGRect,
    divider: CGFloat,
    buttons: &[(&'static str, &'static str)],
) {
    let buttons_row_center = (app_frame.size.height + divider) / 2.0;

    // TODO: more buttons
    let &[(title_text, selector)] = buttons else {
        unreachable!();
    };

    let button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeRoundedRect];
    let button_size = CGSize {
        width: app_frame.size.width - 20.0,
        height: 30.0,
    };
    let button_frame = CGRect {
        origin: CGPoint {
            x: (app_frame.size.width - button_size.width) / 2.0,
            y: buttons_row_center - button_size.height / 2.0,
        },
        size: button_size,
    };
    let text = ns_string::get_static_str(env, title_text);
    () = msg![env; button setTitle:text forState:UIControlStateNormal];
    () = msg![env; button setFrame:button_frame];
    // FIXME: manually calling layoutSubviews shouldn't be needed
    () = msg![env; button layoutSubviews];
    let selector = env.objc.lookup_selector(selector).unwrap();
    () = msg![env; button addTarget:delegate
                             action:selector
                   forControlEvents:UIControlEventTouchUpInside];
    () = msg![env; main_view addSubview:button];
}
