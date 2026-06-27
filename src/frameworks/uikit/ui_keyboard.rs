/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIKeyboard` — keyboard appearance/dismissal stubs and notification
//! constants.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::CGSize;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg_class, nil, objc_classes, ClassExports, TrivialHostObject};

// MARK: - Notification names

pub const UIKeyboardWillShowNotification: &str = "UIKeyboardWillShowNotification";
pub const UIKeyboardDidShowNotification: &str = "UIKeyboardDidShowNotification";
pub const UIKeyboardWillHideNotification: &str = "UIKeyboardWillHideNotification";
pub const UIKeyboardDidHideNotification: &str = "UIKeyboardDidHideNotification";
pub const UIKeyboardWillChangeFrameNotification: &str = "UIKeyboardWillChangeFrameNotification";
pub const UIKeyboardDidChangeFrameNotification: &str = "UIKeyboardDidChangeFrameNotification";

// MARK: - UserInfo keys (iOS 3.2+)

pub const UIKeyboardFrameBeginUserInfoKey: &str = "UIKeyboardFrameBeginUserInfoKey";
pub const UIKeyboardFrameEndUserInfoKey: &str = "UIKeyboardFrameEndUserInfoKey";
pub const UIKeyboardAnimationDurationUserInfoKey: &str = "UIKeyboardAnimationDurationUserInfoKey";
pub const UIKeyboardAnimationCurveUserInfoKey: &str = "UIKeyboardAnimationCurveUserInfoKey";
pub const UIKeyboardIsLocalUserInfoKey: &str = "UIKeyboardIsLocalUserInfoKey";

// MARK: - Legacy userInfo keys (pre-iOS 3.2, still seen in old apps)

pub const UIKeyboardBoundsUserInfoKey: &str = "UIKeyboardBoundsUserInfoKey";
pub const UIKeyboardCenterBeginUserInfoKey: &str = "UIKeyboardCenterBeginUserInfoKey";
pub const UIKeyboardCenterEndUserInfoKey: &str = "UIKeyboardCenterEndUserInfoKey";

// MARK: - Keyboard type / appearance / return key

type UIKeyboardType = NSInteger;
const UIKeyboardTypeDefault: UIKeyboardType = 0;
const UIKeyboardTypeASCIICapable: UIKeyboardType = 1;
const UIKeyboardTypeNumbersAndPunctuation: UIKeyboardType = 2;
const UIKeyboardTypeURL: UIKeyboardType = 3;
const UIKeyboardTypeNumberPad: UIKeyboardType = 4;
const UIKeyboardTypePhonePad: UIKeyboardType = 5;
const UIKeyboardTypeNamePhonePad: UIKeyboardType = 6;
const UIKeyboardTypeEmailAddress: UIKeyboardType = 7;
const UIKeyboardTypeDecimalPad: UIKeyboardType = 8;
const UIKeyboardTypeTwitter: UIKeyboardType = 9;
const UIKeyboardTypeWebSearch: UIKeyboardType = 10;

type UIKeyboardAppearance = NSInteger;
const UIKeyboardAppearanceDefault: UIKeyboardAppearance = 0;
const UIKeyboardAppearanceDark: UIKeyboardAppearance = 1;
const UIKeyboardAppearanceLight: UIKeyboardAppearance = 2;
const UIKeyboardAppearanceAlert: UIKeyboardAppearance = UIKeyboardAppearanceDark;

type UIReturnKeyType = NSInteger;
const UIReturnKeyDefault: UIReturnKeyType = 0;
const UIReturnKeyGo: UIReturnKeyType = 1;
const UIReturnKeyGoogle: UIReturnKeyType = 2;
const UIReturnKeyJoin: UIReturnKeyType = 3;
const UIReturnKeyNext: UIReturnKeyType = 4;
const UIReturnKeyRoute: UIReturnKeyType = 5;
const UIReturnKeySearch: UIReturnKeyType = 6;
const UIReturnKeySend: UIReturnKeyType = 7;
const UIReturnKeyYahoo: UIReturnKeyType = 8;
const UIReturnKeyDone: UIReturnKeyType = 9;
const UIReturnKeyEmergencyCall: UIReturnKeyType = 10;
const UIReturnKeyContinue: UIReturnKeyType = 11;

type UITextAutocapitalizationType = NSInteger;
const UITextAutocapitalizationTypeNone: UITextAutocapitalizationType = 0;
const UITextAutocapitalizationTypeWords: UITextAutocapitalizationType = 1;
const UITextAutocapitalizationTypeSentences: UITextAutocapitalizationType = 2;
const UITextAutocapitalizationTypeAllCharacters: UITextAutocapitalizationType = 3;

type UITextAutocorrectionType = NSInteger;
const UITextAutocorrectionTypeDefault: UITextAutocorrectionType = 0;
const UITextAutocorrectionTypeNo: UITextAutocorrectionType = 1;
const UITextAutocorrectionTypeYes: UITextAutocorrectionType = 2;

type UITextSpellCheckingType = NSInteger;
const UITextSpellCheckingTypeDefault: UITextSpellCheckingType = 0;
const UITextSpellCheckingTypeNo: UITextSpellCheckingType = 1;
const UITextSpellCheckingTypeYes: UITextSpellCheckingType = 2;

pub const CONSTANTS: ConstantExports = &[
    // Notification names
    (
        "_UIKeyboardWillShowNotification",
        HostConstant::NSString(UIKeyboardWillShowNotification),
    ),
    (
        "_UIKeyboardDidShowNotification",
        HostConstant::NSString(UIKeyboardDidShowNotification),
    ),
    (
        "_UIKeyboardWillHideNotification",
        HostConstant::NSString(UIKeyboardWillHideNotification),
    ),
    (
        "_UIKeyboardDidHideNotification",
        HostConstant::NSString(UIKeyboardDidHideNotification),
    ),
    (
        "_UIKeyboardWillChangeFrameNotification",
        HostConstant::NSString(UIKeyboardWillChangeFrameNotification),
    ),
    (
        "_UIKeyboardDidChangeFrameNotification",
        HostConstant::NSString(UIKeyboardDidChangeFrameNotification),
    ),
    // UserInfo keys
    (
        "_UIKeyboardFrameBeginUserInfoKey",
        HostConstant::NSString(UIKeyboardFrameBeginUserInfoKey),
    ),
    (
        "_UIKeyboardFrameEndUserInfoKey",
        HostConstant::NSString(UIKeyboardFrameEndUserInfoKey),
    ),
    (
        "_UIKeyboardAnimationDurationUserInfoKey",
        HostConstant::NSString(UIKeyboardAnimationDurationUserInfoKey),
    ),
    (
        "_UIKeyboardAnimationCurveUserInfoKey",
        HostConstant::NSString(UIKeyboardAnimationCurveUserInfoKey),
    ),
    (
        "_UIKeyboardIsLocalUserInfoKey",
        HostConstant::NSString(UIKeyboardIsLocalUserInfoKey),
    ),
    // Legacy keys
    (
        "_UIKeyboardBoundsUserInfoKey",
        HostConstant::NSString(UIKeyboardBoundsUserInfoKey),
    ),
    (
        "_UIKeyboardCenterBeginUserInfoKey",
        HostConstant::NSString(UIKeyboardCenterBeginUserInfoKey),
    ),
    (
        "_UIKeyboardCenterEndUserInfoKey",
        HostConstant::NSString(UIKeyboardCenterEndUserInfoKey),
    ),
];

/// State for the private keyboard classes. These expose `+sharedInstance`,
/// which (like all Cocoa singletons) must return the *same* object on every
/// call. The previous implementation allocated a brand-new object each time,
/// so old apps that call `[UIKeyboard sharedInstance]` (or
/// `[UIKeyboardImpl sharedInstance]`) repeatedly — sometimes recursively, via
/// `+initialize`/`retain` chains — kept manufacturing fresh objects. That
/// churn is what tripped the `objc_msgSend` recursion guard (bailing out with
/// a nil return) and left the app messaging a half-built/freed object.
#[derive(Default)]
pub struct State {
    ui_keyboard_shared_instance: Option<id>,
    ui_keyboard_impl_shared_instance: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// MARK: - UIKeyboard (private internal class, seen in old apps via NIBs)

@implementation UIKeyboard: UIView

+ (id)sharedInstance {
    if let Some(existing) = env.framework_state.uikit.ui_keyboard.ui_keyboard_shared_instance {
        return existing;
    }
    let instance = env.objc.alloc_static_object(this, Box::new(TrivialHostObject), &mut env.mem);
    env.framework_state.uikit.ui_keyboard.ui_keyboard_shared_instance = Some(instance);
    instance
}

+ (bool)isInHardwareKeyboardMode {
    false
}

+ (bool)isOnScreen {
    false
}

+ (CGSize)defaultSizeForOrientation:(NSInteger)_orientation {
    // Standard iPhone keyboard size in portrait.
    CGSize { width: 320.0, height: 216.0 }
}

+ (CGSize)defaultSizeForInterfaceOrientation:(NSInteger)_orientation {
    CGSize { width: 320.0, height: 216.0 }
}

- (())orderInWithAnimation:(bool)_animated {
    log!("UIKeyboard orderInWithAnimation: stubbed (no keyboard shown)");
}

- (())orderOutWithAnimation:(bool)_animated {
    log!("UIKeyboard orderOutWithAnimation: stubbed");
}

- (())activate {
    log!("UIKeyboard activate: stubbed");
}

- (())deactivate {
    log!("UIKeyboard deactivate: stubbed");
}

- (bool)isVisible {
    false
}

@end

// MARK: - UIKeyboardImpl (private, accessed by some apps)

@implementation UIKeyboardImpl: NSObject

+ (id)sharedInstance {
    if let Some(existing) = env.framework_state.uikit.ui_keyboard.ui_keyboard_impl_shared_instance {
        return existing;
    }
    let instance = env.objc.alloc_static_object(this, Box::new(TrivialHostObject), &mut env.mem);
    env.framework_state.uikit.ui_keyboard.ui_keyboard_impl_shared_instance = Some(instance);
    instance
}

+ (id)activeInstance {
    nil
}

- (())setDelegate:(id)_delegate {
    log!("UIKeyboardImpl setDelegate: stubbed");
}

- (id)delegate {
    nil
}

- (())setReturnKeyType:(UIReturnKeyType)_type {
    // Stub.
}

- (UIReturnKeyType)returnKeyType {
    UIReturnKeyDefault
}

- (())setKeyboardType:(UIKeyboardType)_type {
    // Stub.
}

- (UIKeyboardType)keyboardType {
    UIKeyboardTypeDefault
}

- (())setKeyboardAppearance:(UIKeyboardAppearance)_appearance {
    // Stub.
}

- (UIKeyboardAppearance)keyboardAppearance {
    UIKeyboardAppearanceDefault
}

- (())setAutocorrectionType:(UITextAutocorrectionType)_type {
    // Stub.
}

- (UITextAutocorrectionType)autocorrectionType {
    UITextAutocorrectionTypeDefault
}

- (())setAutocapitalizationType:(UITextAutocapitalizationType)_type {
    // Stub.
}

- (UITextAutocapitalizationType)autocapitalizationType {
    UITextAutocapitalizationTypeNone
}

- (())setSpellCheckingType:(UITextSpellCheckingType)_type {
    // Stub.
}

- (UITextSpellCheckingType)spellCheckingType {
    UITextSpellCheckingTypeDefault
}

- (())setSecureTextEntry:(bool)_secure {
    // Stub.
}

- (bool)isSecureTextEntry {
    false
}

- (())updateForChangedSelection {
    // Stub.
}

- (())clearAutocorrection {
    // Stub.
}

@end

// MARK: - UITextInputTraits category stub (implemented as a standalone class
//          so other classes can reference these property names)

@implementation UITextInputTraits: NSObject
@end

};

/// Helper called by text-input views (UITextField, UITextView) to post the
/// standard keyboard-will/did-show notifications with an empty frame userInfo.
/// In touchHLE we never actually show a keyboard, but apps that observe these
/// notifications (to scroll their content) need them to fire.
pub fn post_keyboard_notifications(env: &mut crate::Environment, will_show: bool) {
    use crate::frameworks::foundation::ns_string::get_static_str;
    use crate::objc::msg;

    let (will_name, did_name) = if will_show {
        (
            UIKeyboardWillShowNotification,
            UIKeyboardDidShowNotification,
        )
    } else {
        (
            UIKeyboardWillHideNotification,
            UIKeyboardDidHideNotification,
        )
    };

    let app: crate::objc::id = msg_class![env; UIApplication sharedApplication];
    let center: crate::objc::id = msg_class![env; NSNotificationCenter defaultCenter];

    let user_info: crate::objc::id = msg_class![env; NSMutableDictionary new];

    for name in [will_name, did_name] {
        let ns_name = get_static_str(env, name);
        let _: () = msg![env; center postNotificationName:ns_name
                                                   object:app
                                                 userInfo:user_info];
    }

    crate::objc::release(env, user_info);
}
