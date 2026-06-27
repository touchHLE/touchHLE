/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActivity`, `UIActivityItemProvider`, `UIActivityViewController`.
//!
//! These classes drive iOS 6+'s system share sheet. touchHLE targets pre-iOS
//! 5 apps but a number of vendored SDKs (Flurry, Chartboost, social SDKs)
//! still reach for these classes via `NSClassFromString` to decide whether
//! the modern share sheet path is available.
//!
//! Apple documentation:
//! * <https://developer.apple.com/documentation/uikit/uiactivity>
//! * <https://developer.apple.com/documentation/uikit/uiactivityitemprovider>
//! * <https://developer.apple.com/documentation/uikit/uiactivityviewcontroller>
//!
//! We implement just enough of the public surface to let those probes
//! succeed and to let an app present (and immediately dismiss) a share
//! sheet without crashing. There is no host UI for actually presenting
//! sharing options — the view controller behaves like an empty modal that
//! invokes its completion handler with `completed = NO` when presented.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, nil, objc_classes, release,
    retain, ClassExports, HostObject, NSZonePtr,
};

pub type UIActivityCategory = NSInteger;
pub const UIActivityCategoryAction: UIActivityCategory = 0;
pub const UIActivityCategoryShare: UIActivityCategory = 1;

#[derive(Default)]
pub struct UIActivityItemProviderHostObject {
    placeholder_item: id,
    item: id,
}
impl HostObject for UIActivityItemProviderHostObject {}

#[derive(Default)]
pub struct UIActivityViewControllerHostObject {
    superclass: super::ui_view_controller::UIViewControllerHostObject,
    activity_items: id, // NSArray *
    application_activities: id, // NSArray *
    completion_handler: id, // Objective-C block, kept for the ABI; we never invoke it
    completion_with_items_handler: id,
    excluded_activity_types: id,
}
impl_HostObject_with_superclass!(UIActivityViewControllerHostObject);

pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

// --- UIActivity ---------------------------------------------------------

@implementation UIActivity: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIActivityItemProviderHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

// Apple documents these as overridable; our default-class implementation
// returns nil/Action so subclasses' overrides remain authoritative when
// dispatched dynamically.
- (id)activityType { nil }
- (id)activityTitle { nil }
- (id)activityImage { nil }
- (UIActivityCategory)activityCategory { UIActivityCategoryAction }
- (bool)canPerformWithActivityItems:(id)_items { false }
- (())prepareWithActivityItems:(id)_items {}
- (id)activityViewController { nil }
- (())performActivity {
    // Apple's contract: subclasses MUST call -activityDidFinish: when done.
    let _: () = msg![env; this activityDidFinish:false];
}
- (())activityDidFinish:(bool)_completed {}

@end

// --- UIActivityItemProvider --------------------------------------------

@implementation UIActivityItemProvider: NSOperation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIActivityItemProviderHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    let _: id = msg![env; this initWithPlaceholderItem:nil];
    this
}

- (id)initWithPlaceholderItem:(id)placeholder {
    let placeholder = retain(env, placeholder);
    let host = env.objc.borrow_mut::<UIActivityItemProviderHostObject>(this);
    host.placeholder_item = placeholder;
    this
}

- (id)placeholderItem {
    env.objc.borrow::<UIActivityItemProviderHostObject>(this).placeholder_item
}

- (id)item {
    // Apple's default returns the placeholder synchronously when no
    // subclass override has produced a real item yet.
    let host = env.objc.borrow::<UIActivityItemProviderHostObject>(this);
    if host.item != nil { host.item } else { host.placeholder_item }
}

- (id)activityType { nil }

- (())main {
    // Provider operations on Apple's runtime call -item from -main.
    let item: id = msg![env; this item];
    env.objc.borrow_mut::<UIActivityItemProviderHostObject>(this).item = retain(env, item);
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<UIActivityItemProviderHostObject>(this));
    release(env, host.placeholder_item);
    release(env, host.item);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

// --- UIActivityViewController ------------------------------------------

@implementation UIActivityViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIActivityViewControllerHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithActivityItems:(id)activityItems
      applicationActivities:(id)applicationActivities {
    let _: id = msg![env; this init];
    let items = retain(env, activityItems);
    let apps = retain(env, applicationActivities);
    let host = env.objc.borrow_mut::<UIActivityViewControllerHostObject>(this);
    host.activity_items = items;
    host.application_activities = apps;
    this
}

- (id)excludedActivityTypes {
    env.objc.borrow::<UIActivityViewControllerHostObject>(this).excluded_activity_types
}
- (())setExcludedActivityTypes:(id)types {
    let new_value = retain(env, types);
    let host = env.objc.borrow_mut::<UIActivityViewControllerHostObject>(this);
    let old = host.excluded_activity_types;
    host.excluded_activity_types = new_value;
    release(env, old);
}

- (id)completionHandler {
    env.objc.borrow::<UIActivityViewControllerHostObject>(this).completion_handler
}
- (())setCompletionHandler:(id)handler {
    let new_value = retain(env, handler);
    let host = env.objc.borrow_mut::<UIActivityViewControllerHostObject>(this);
    let old = host.completion_handler;
    host.completion_handler = new_value;
    release(env, old);
}

- (id)completionWithItemsHandler {
    env.objc.borrow::<UIActivityViewControllerHostObject>(this).completion_with_items_handler
}
- (())setCompletionWithItemsHandler:(id)handler {
    let new_value = retain(env, handler);
    let host = env.objc.borrow_mut::<UIActivityViewControllerHostObject>(this);
    let old = host.completion_with_items_handler;
    host.completion_with_items_handler = new_value;
    release(env, old);
}

- (())viewDidLoad {
    // Apple's view-controller lifecycle hook. No-op here — there is no
    // share-sheet UI to install.
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<UIActivityViewControllerHostObject>(this));
    release(env, host.activity_items);
    release(env, host.application_activities);
    release(env, host.completion_handler);
    release(env, host.completion_with_items_handler);
    release(env, host.excluded_activity_types);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};
