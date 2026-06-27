/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIStoryboard`, `UIStoryboardSegue` and `UIStoryboardPlaceholder`.
//!
//! Storyboards (iOS 5+) replace the legacy `NSMainNibFile` workflow with a
//! compiled `*.storyboardc` bundle directory that contains:
//!
//!   * an `Info.plist` describing the scene graph (most importantly the
//!     `UIStoryboardDesignatedEntryPointIdentifier` and the
//!     `UIViewControllerIdentifiersToNibNames` lookup table), and
//!   * one nib directory per scene (e.g. a `UIViewController-XYZ.nib/` for
//!     the view controller, plus a separate `XYZ-view-ABC.nib/` for the
//!     view it manages).
//!
//! Inside each scene nib the file's owner is a special
//! `UIStoryboardPlaceholder` proxy. The view controller's `view` outlet is
//! wired up to that placeholder so the actual view nib (the second
//! `*.nib/` directory above) can be loaded lazily on first access.
//!
//! References:
//! * Apple, "Introducing Storyboards", _Document Outline_, 2011.
//! * `ibtool` output for any storyboard exposed via `unzip *.ipa`.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window::DeviceFamily;
use crate::Environment;

/// Plain-string constants exported so guest binaries that import them via
/// `__nl_symbol_ptr` resolve to a stable `NSString*` (Apple uses these as
/// keys on the `UIStoryboardSegue` user-info dictionary).
pub const UIStoryboardSegueDestinationKey: &str = "destination";
pub const UIStoryboardSegueSourceKey: &str = "source";

pub const CONSTANTS: ConstantExports = &[
    (
        "_UIStoryboardSegueDestinationKey",
        HostConstant::NSString(UIStoryboardSegueDestinationKey),
    ),
    (
        "_UIStoryboardSegueSourceKey",
        HostConstant::NSString(UIStoryboardSegueSourceKey),
    ),
];

/// Per-storyboard host state captured at `+storyboardWithName:bundle:` time.
#[derive(Default)]
struct UIStoryboardHostObject {
    /// User-visible name (e.g. `"Main"`).
    /// `NSString*` (retained).
    name: id,
    /// Bundle the storyboard was loaded from (retained).
    /// `NSBundle*`
    bundle: id,
    /// Path of the per-device `*.storyboardc` containing the metadata,
    /// relative to the bundle's resource path (e.g.
    /// `Base.lproj/Main~iphone.storyboardc`).
    storyboardc_relative_dir: String,
    /// Identifier of the designated entry-point scene, or empty when the
    /// storyboard has no initial view controller.
    initial_identifier: String,
    /// `UIViewControllerIdentifiersToNibNames` from the storyboardc
    /// metadata: maps each scene identifier to its base nib name (without
    /// device suffix or `.nib` extension).
    identifier_to_nib: std::collections::HashMap<String, String>,
}
impl HostObject for UIStoryboardHostObject {}

/// Placeholder used by storyboard-loaded view controllers as a deferred
/// stand-in for the actual `UIView`. The view nib is referenced by name
/// and loaded lazily the first time `-[UIViewController view]` is read.
///
/// At nib-loading time, the placeholder is supplied to `UINib`'s
/// `UIProxiedObjectsKey` external-objects table under the identifier
/// `"UIStoryboardPlaceholder"`; the scene nib's outlet connection then
/// sets the placeholder as the view controller's `view`. We intercept
/// that assignment inside `-[UIViewController setView:]` and capture the
/// placeholder's metadata so the view can be loaded lazily on first
/// access.
#[derive(Default)]
pub(crate) struct UIStoryboardPlaceholderHostObject {
    /// `NSString*` containing the storyboardc directory relative to the
    /// bundle's resource path (retained). Used to construct the view
    /// nib's full resource path: `<storyboardc>/<view_nib_name>`.
    pub storyboardc_relative_dir: id,
}
impl HostObject for UIStoryboardPlaceholderHostObject {}

/// Per-segue host state.
#[derive(Default)]
struct UIStoryboardSegueHostObject {
    identifier: id,
    source: id,
    destination: id,
}
impl HostObject for UIStoryboardSegueHostObject {}

/// Locate the `*.storyboardc` directory for the given storyboard `name`,
/// taking the device-specific (`~iphone`/`~ipad`) variants and the
/// `Base.lproj` Base-internationalization fallback into account.
///
/// Returns `(absolute_path, bundle_relative_path)` on success.
fn locate_storyboardc(
    env: &mut Environment,
    bundle: id,
    name: &str,
    device_family: Option<DeviceFamily>,
) -> Option<(String, String)> {
    let device_suffix = match device_family {
        Some(df) if df.is_ipad() => Some("~ipad"),
        Some(_) => Some("~iphone"),
        None => None,
    };

    // Try device-specific then plain; NSBundle.pathForResource: already
    // walks per-language and Base.lproj automatically.
    let mut candidates: Vec<String> = Vec::new();
    if let Some(suffix) = device_suffix {
        candidates.push(format!("{}{}", name, suffix));
    }
    candidates.push(name.to_string());

    let type_ns: id = get_static_str(env, "storyboardc");

    let resource_path: id = msg![env; bundle resourcePath];
    let bundle_resource_path = to_rust_string(env, resource_path).into_owned();

    for cand in &candidates {
        let cand_ns: id = from_rust_string(env, cand.clone());
        let path: id = msg![env; bundle pathForResource:cand_ns ofType:type_ns];
        release(env, cand_ns);
        if path != nil {
            let absolute = to_rust_string(env, path).into_owned();
            let relative = if let Some(stripped) =
                absolute.strip_prefix(&format!("{}/", bundle_resource_path))
            {
                stripped.to_string()
            } else {
                absolute.clone()
            };
            return Some((absolute, relative));
        }
    }
    None
}

/// Read the storyboardc metadata (`Info.plist` or, on iOS 8+, the
/// `Info-8.0+.plist` variant — both contain the same keys) and turn it
/// into the in-memory descriptors expected by `UIStoryboard`.
fn load_storyboardc_metadata(
    env: &mut Environment,
    storyboardc_dir: &str,
) -> Option<(String, std::collections::HashMap<String, String>)> {
    let mut dict: id = nil;
    for filename in ["Info-8.0+.plist", "Info.plist"] {
        let path = format!("{}/{}", storyboardc_dir, filename);
        let path_ns: id = from_rust_string(env, path);
        let attempt: id = msg_class![env; NSDictionary alloc];
        let attempt: id = msg![env; attempt initWithContentsOfFile:path_ns];
        release(env, path_ns);
        if attempt != nil {
            dict = attempt;
            break;
        }
    }

    if dict == nil {
        log!(
            "Warning: UIStoryboard couldn't read storyboardc metadata at {:?}",
            storyboardc_dir
        );
        return None;
    }

    let entry_key: id = get_static_str(env, "UIStoryboardDesignatedEntryPointIdentifier");
    let entry_id_ns: id = msg![env; dict objectForKey:entry_key];
    let initial_identifier = if entry_id_ns != nil {
        to_rust_string(env, entry_id_ns).into_owned()
    } else {
        String::new()
    };

    let map_key: id = get_static_str(env, "UIViewControllerIdentifiersToNibNames");
    let map: id = msg![env; dict objectForKey:map_key];
    let mut identifier_to_nib: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if map != nil {
        let keys: id = msg![env; map allKeys];
        let count: crate::frameworks::foundation::NSUInteger = msg![env; keys count];
        for i in 0..count {
            let key_ns: id = msg![env; keys objectAtIndex:i];
            let value_ns: id = msg![env; map objectForKey:key_ns];
            if key_ns != nil && value_ns != nil {
                let key = to_rust_string(env, key_ns).into_owned();
                let value = to_rust_string(env, value_ns).into_owned();
                identifier_to_nib.insert(key, value);
            }
        }
    }

    release(env, dict);
    Some((initial_identifier, identifier_to_nib))
}

/// Build the per-scene nib name relative to the bundle's resource path.
/// Returns `None` if no matching `.nib/` directory exists.
///
/// Storyboards usually ship the per-device nibs inside
/// `Main.storyboardc/<scene>~<device>.nib/`, while the metadata lives in
/// a `Main~<device>.storyboardc/` (with no nibs of its own). We try both
/// the canonical and the `Main`-without-device variant, with and without
/// the device suffix.
fn find_scene_nib_name(
    env: &mut Environment,
    bundle: id,
    storyboardc_relative_dir: &str,
    nib_base_name: &str,
    device_family: Option<DeviceFamily>,
) -> Option<String> {
    let device_suffix = match device_family {
        Some(df) if df.is_ipad() => "~ipad",
        Some(_) => "~iphone",
        None => "",
    };

    let mut storyboardc_candidates = vec![storyboardc_relative_dir.to_string()];
    // Strip the device suffix from the storyboardc dir if present, e.g.
    // `Base.lproj/Main~iphone.storyboardc` -> `Base.lproj/Main.storyboardc`.
    if let Some(stripped) = storyboardc_relative_dir.strip_suffix("~iphone.storyboardc") {
        storyboardc_candidates.push(format!("{}.storyboardc", stripped));
    } else if let Some(stripped) = storyboardc_relative_dir.strip_suffix("~ipad.storyboardc") {
        storyboardc_candidates.push(format!("{}.storyboardc", stripped));
    }

    let resource_path: id = msg![env; bundle resourcePath];
    let bundle_resource_path = to_rust_string(env, resource_path).into_owned();
    let file_manager: id = msg_class![env; NSFileManager defaultManager];

    for sb_dir in &storyboardc_candidates {
        let mut nib_candidates: Vec<String> = Vec::new();
        if !device_suffix.is_empty() {
            nib_candidates.push(format!(
                "{}/{}{}.nib",
                sb_dir, nib_base_name, device_suffix
            ));
        }
        nib_candidates.push(format!("{}/{}.nib", sb_dir, nib_base_name));

        for cand in nib_candidates {
            let absolute = format!("{}/{}", bundle_resource_path, cand);
            let absolute_ns: id = from_rust_string(env, absolute);
            let exists: bool = msg![env; file_manager fileExistsAtPath:absolute_ns];
            release(env, absolute_ns);
            if exists {
                // Strip trailing `.nib` so it can be passed as a nib name
                // to `-[UINib pathForResource:ofType:"nib"]`.
                return Some(cand.trim_end_matches(".nib").to_string());
            }
        }
    }
    None
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - UIStoryboard
// =========================================================================

@implementation UIStoryboard: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIStoryboardHostObject {
        name: nil,
        bundle: nil,
        storyboardc_relative_dir: String::new(),
        initial_identifier: String::new(),
        identifier_to_nib: std::collections::HashMap::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)storyboardWithName:(id)name bundle:(id)bundle { // NSString*, NSBundle*
    if name == nil {
        log!("Warning: +[UIStoryboard storyboardWithName:nil bundle:] called");
        return nil;
    }

    let effective_bundle: id = if bundle == nil {
        msg_class![env; NSBundle mainBundle]
    } else {
        bundle
    };

    let name_str = to_rust_string(env, name).into_owned();

    let device_family = env.options.device_family;
    let (storyboardc_absolute, storyboardc_relative) =
        match locate_storyboardc(env, effective_bundle, &name_str, device_family) {
            Some(paths) => paths,
            None => {
                log!(
                    "Warning: +[UIStoryboard storyboardWithName:{:?} bundle:{:?}] storyboardc not found",
                    name_str,
                    bundle,
                );
                return nil;
            }
        };

    let (initial_identifier, identifier_to_nib) =
        match load_storyboardc_metadata(env, &storyboardc_absolute) {
            Some(meta) => meta,
            None => return nil,
        };

    let storyboard: id = msg![env; this alloc];

    retain(env, name);
    retain(env, effective_bundle);

    {
        let host_obj = env.objc.borrow_mut::<UIStoryboardHostObject>(storyboard);
        host_obj.name = name;
        host_obj.bundle = effective_bundle;
        host_obj.storyboardc_relative_dir = storyboardc_relative;
        host_obj.initial_identifier = initial_identifier;
        host_obj.identifier_to_nib = identifier_to_nib;
    }

    autorelease(env, storyboard)
}

- (id)name {
    env.objc.borrow::<UIStoryboardHostObject>(this).name
}

- (())dealloc {
    let (name, bundle) = {
        let host = env.objc.borrow::<UIStoryboardHostObject>(this);
        (host.name, host.bundle)
    };
    if name != nil { release(env, name); }
    if bundle != nil { release(env, bundle); }
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)instantiateInitialViewController {
    let identifier = env
        .objc
        .borrow::<UIStoryboardHostObject>(this)
        .initial_identifier
        .clone();
    if identifier.is_empty() {
        log!("Warning: UIStoryboard has no designated entry point");
        return nil;
    }
    let identifier_ns = from_rust_string(env, identifier);
    let vc: id = msg![env; this instantiateViewControllerWithIdentifier:identifier_ns];
    release(env, identifier_ns);
    vc
}

- (id)instantiateViewControllerWithIdentifier:(id)identifier {
    if identifier == nil {
        return nil;
    }
    let identifier_str = to_rust_string(env, identifier).into_owned();
    let (storyboardc_relative_dir, vc_nib_base_name, bundle) = {
        let host = env.objc.borrow::<UIStoryboardHostObject>(this);
        let nib_base = match host.identifier_to_nib.get(&identifier_str) {
            Some(n) => n.clone(),
            None => {
                log!(
                    "Warning: UIStoryboard has no scene with identifier {:?}",
                    identifier_str
                );
                return nil;
            }
        };
        (
            host.storyboardc_relative_dir.clone(),
            nib_base,
            host.bundle,
        )
    };

    let device_family = env.options.device_family;
    let scene_nib_relative_no_ext = match find_scene_nib_name(
        env,
        bundle,
        &storyboardc_relative_dir,
        &vc_nib_base_name,
        device_family,
    ) {
        Some(p) => p,
        None => {
            log!(
                "Warning: UIStoryboard couldn't find scene nib {:?} in {:?}",
                vc_nib_base_name,
                storyboardc_relative_dir,
            );
            return nil;
        }
    };

    // The directory that actually contains the scene nibs may differ from
    // the storyboardc that holds the metadata. The canonical layout used
    // by Xcode-compiled storyboards keeps the nibs in the universal
    // `Main.storyboardc/` while the metadata lives in the device-specific
    // `Main~iphone.storyboardc/` (or vice versa). The view nibs that
    // -[UIViewController loadView] needs to look up later live in the
    // same directory as the scene nib that we just located.
    let view_nib_dir = scene_nib_relative_no_ext
        .rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_else(|| storyboardc_relative_dir.clone());

    // Build the placeholder that the scene nib's `view` outlet binds to.
    let placeholder_class = env.objc.get_known_class("UIStoryboardPlaceholder", &mut env.mem);
    let placeholder: id = msg![env; placeholder_class alloc];
    let placeholder: id = msg![env; placeholder init];

    // Remember the directory containing the view nibs on the placeholder
    // so that the view controller can recover the path used to load the
    // view nib.
    let storyboardc_dir_ns = from_rust_string(env, view_nib_dir.clone());
    retain(env, storyboardc_dir_ns); // explicit +1 held by host
    {
        let host = env
            .objc
            .borrow_mut::<UIStoryboardPlaceholderHostObject>(placeholder);
        host.storyboardc_relative_dir = storyboardc_dir_ns;
    }

    // Load the scene nib using the placeholder as the substitute for the
    // `UIStoryboardPlaceholder` proxy AND for `IBFilesOwner` (Apple's
    // runtime passes the placeholder as the file's owner so the scene
    // nib's view-controller class swapper has somewhere to attach to).
    let external_objects: id = msg_class![env; NSMutableDictionary dictionary];
    let placeholder_key: id = get_static_str(env, "UIStoryboardPlaceholder");
    let _: () = msg![env; external_objects setObject:placeholder forKey:placeholder_key];
    let owner_key: id = get_static_str(env, "IBFilesOwner");
    let _: () = msg![env; external_objects setObject:placeholder forKey:owner_key];

    let options: id = msg_class![env; NSMutableDictionary dictionary];
    let proxied_key: id =
        get_static_str(env, crate::frameworks::uikit::ui_nib::UINibProxiedObjectsKey);
    let _: () = msg![env; options setObject:external_objects forKey:proxied_key];

    let scene_nib_name_ns: id = from_rust_string(env, scene_nib_relative_no_ext.clone());
    let ui_nib: id = msg_class![env; UINib nibWithNibName:scene_nib_name_ns bundle:bundle];
    let top_level: id = msg![env; ui_nib instantiateWithOwner:placeholder options:options];

    // Find the top-level UIViewController among the decoded objects.
    let vc_class = env.objc.get_known_class("UIViewController", &mut env.mem);
    let mut vc: id = nil;
    if top_level != nil {
        let count: crate::frameworks::foundation::NSUInteger = msg![env; top_level count];
        for i in 0..count {
            let obj: id = msg![env; top_level objectAtIndex:i];
            if obj == nil { continue; }
            let obj_class = msg![env; obj class];
            if env.objc.class_is_subclass_of(obj_class, vc_class) {
                vc = obj;
                break;
            }
        }
    }

    if vc != nil {
        // Wire up the storyboard back-pointer so `-[UIViewController
        // storyboard]` returns this object for segue resolution.
        crate::frameworks::uikit::ui_view_controller::set_storyboard(env, vc, this);
        // Promote the placeholder marker on `vc.view` to a proper
        // "load this nib from this directory when -view is queried"
        // record stored directly on the view controller.
        crate::frameworks::uikit::ui_view_controller::promote_storyboard_placeholder(
            env,
            vc,
            &view_nib_dir,
        );
    } else {
        log!(
            "Warning: UIStoryboard scene {:?} did not produce a UIViewController",
            identifier_str
        );
    }

    release(env, placeholder);
    release(env, scene_nib_name_ns);

    if vc != nil {
        retain(env, vc);
        autorelease(env, vc)
    } else {
        nil
    }
}

@end

// =========================================================================
// MARK: - UIStoryboardPlaceholder
// =========================================================================

@implementation UIStoryboardPlaceholder: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UIStoryboardPlaceholderHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    this
}

- (id)initWithCoder:(id)_coder {
    // Storyboard placeholders are normally instantiated via the external
    // objects table, not via the unarchiver — but a malformed nib that
    // somehow encodes one shouldn't crash the emulator.
    this
}

- (())dealloc {
    let storyboardc_dir = env
        .objc
        .borrow::<UIStoryboardPlaceholderHostObject>(this)
        .storyboardc_relative_dir;
    if storyboardc_dir != nil { release(env, storyboardc_dir); }
    env.objc.dealloc_object(this, &mut env.mem)
}

// `UIRuntimeOutletConnection.-connect` walks all outlets including ones
// targeting the placeholder; this trap absorbs any KVC-based connection
// without raising `setValue:forUndefinedKey:`.
- (())setValue:(id)_value forKey:(id)_key {
}

@end

// =========================================================================
// MARK: - UIStoryboardSegue
// =========================================================================

@implementation UIStoryboardSegue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UIStoryboardSegueHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)initWithIdentifier:(id)identifier
                  source:(id)source
             destination:(id)destination {
    if identifier != nil { retain(env, identifier); }
    if source != nil { retain(env, source); }
    if destination != nil { retain(env, destination); }
    let host = env.objc.borrow_mut::<UIStoryboardSegueHostObject>(this);
    host.identifier = identifier;
    host.source = source;
    host.destination = destination;
    this
}

- (id)identifier {
    env.objc.borrow::<UIStoryboardSegueHostObject>(this).identifier
}

- (id)sourceViewController {
    env.objc.borrow::<UIStoryboardSegueHostObject>(this).source
}

- (id)destinationViewController {
    env.objc.borrow::<UIStoryboardSegueHostObject>(this).destination
}

- (())perform {
    // The default Apple implementation walks the responder chain to find
    // the navigation/tab/presented controller appropriate for the segue's
    // kind. touchHLE supports the most common case: "show" via a
    // navigation controller, with modal presentation as the fallback.
    let (source, destination) = {
        let h = env.objc.borrow::<UIStoryboardSegueHostObject>(this);
        (h.source, h.destination)
    };
    if source == nil || destination == nil {
        return;
    }
    let nav: id = msg![env; source navigationController];
    if nav != nil {
        let _: () = msg![env; nav pushViewController:destination animated:true];
    } else {
        let _: () = msg![env; source presentModalViewController:destination animated:true];
    }
}

- (())dealloc {
    let (identifier, source, destination) = {
        let h = env.objc.borrow::<UIStoryboardSegueHostObject>(this);
        (h.identifier, h.source, h.destination)
    };
    if identifier != nil { release(env, identifier); }
    if source != nil { release(env, source); }
    if destination != nil { release(env, destination); }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
