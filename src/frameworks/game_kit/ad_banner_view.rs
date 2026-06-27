/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ADBannerView` — iAd framework stub.
//! Apps that use iAd will never show real ads in touchHLE, but we need
//! to stub the class so they don't crash on startup.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, nil, objc_classes, release, retain, ClassExports, NSZonePtr,
};

// `ADBannerContentSizeIdentifier*` CFString constants. iAd-using apps export
// these in `__nl_symbol_ptr` even when they never actually display a banner;
// without them the app sees a NULL CFStringRef and crashes when comparing it
// with `[banner currentContentSizeIdentifier]`. Values are the documented
// content-size strings from the iAd headers.
pub const ADBannerContentSizeIdentifier320x50: &str = "ADBannerContentSizeIdentifier320x50";
pub const ADBannerContentSizeIdentifier480x32: &str = "ADBannerContentSizeIdentifier480x32";
pub const ADBannerContentSizeIdentifierPortrait: &str = "ADBannerContentSizeIdentifierPortrait";
pub const ADBannerContentSizeIdentifierLandscape: &str = "ADBannerContentSizeIdentifierLandscape";

pub const CONSTANTS: ConstantExports = &[
    (
        "_ADBannerContentSizeIdentifier320x50",
        HostConstant::NSString(ADBannerContentSizeIdentifier320x50),
    ),
    (
        "_ADBannerContentSizeIdentifier480x32",
        HostConstant::NSString(ADBannerContentSizeIdentifier480x32),
    ),
    (
        "_ADBannerContentSizeIdentifierPortrait",
        HostConstant::NSString(ADBannerContentSizeIdentifierPortrait),
    ),
    (
        "_ADBannerContentSizeIdentifierLandscape",
        HostConstant::NSString(ADBannerContentSizeIdentifierLandscape),
    ),
    // ADErrorDomain — apps that bind to iAd at link time pull this in.
    ("_ADErrorDomain", HostConstant::NSString("ADErrorDomain")),
];

type ADAdType = NSInteger;
const ADAdTypeBanner: ADAdType = 0;
const ADAdTypeMediumRectangle: ADAdType = 1;

type ADBannerContentSizeIdentifier = id; // NSString*

// MARK: - Host Objects (Moved OUTSIDE the macro)

#[derive(Default)]
struct ADBannerViewHostObject {
    delegate: id,
    ad_type: ADAdType,
    banner_loaded: bool,
    action_in_progress: bool,
    required_content_size_identifiers: id,
    current_content_size_identifier: id,
}
impl crate::objc::HostObject for ADBannerViewHostObject {}

#[derive(Default)]
struct ADInterstitialAdHostObject {
    delegate: id,
    loaded: bool,
}
impl crate::objc::HostObject for ADInterstitialAdHostObject {}

// MARK: - Classes

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// MARK: - ADBannerView

@implementation ADBannerView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ADBannerViewHostObject {
        delegate: nil,
        ad_type: ADAdTypeBanner,
        banner_loaded: false,
        action_in_progress: false,
        required_content_size_identifiers: nil,
        current_content_size_identifier: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    // Immediately tell the delegate no ad is available.
    this
}

- (id)initWithAdType:(ADAdType)ad_type {
    env.objc.borrow_mut::<ADBannerViewHostObject>(this).ad_type = ad_type;
    this
}

- (id)initWithFrame:(CGRect)_frame {
    this
}

- (id)initWithCoder:(id)_coder {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<ADBannerViewHostObject>(this);
    let (delegate, required, current) = (
        host.delegate,
        host.required_content_size_identifiers,
        host.current_content_size_identifier,
    );
    release(env, required);
    release(env, current);
    // delegate is weak — no release
    let _ = delegate;
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<ADBannerViewHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<ADBannerViewHostObject>(this).delegate = delegate;
    // Immediately notify delegate that no ad is available.
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "bannerView:didFailToReceiveAdWithError:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            // Pass nil as the error — apps typically just hide the banner.
            () = msg![env; delegate bannerView:this didFailToReceiveAdWithError:nil];
        }
    }
}

// MARK: - Ad type

- (ADAdType)adType {
    env.objc.borrow::<ADBannerViewHostObject>(this).ad_type
}

// MARK: - State

- (bool)isBannerLoaded {
    false // iAd never loads in touchHLE
}

- (bool)isBannerViewActionInProgress {
    false
}

// MARK: - Content size identifiers (deprecated API, pre-iOS 6)

- (id)requiredContentSizeIdentifiers { // NSSet*
    env.objc.borrow::<ADBannerViewHostObject>(this).required_content_size_identifiers
}

- (())setRequiredContentSizeIdentifiers:(id)identifiers {
    let old = env.objc.borrow::<ADBannerViewHostObject>(this)
        .required_content_size_identifiers;
    release(env, old);
    retain(env, identifiers);
    env.objc.borrow_mut::<ADBannerViewHostObject>(this)
        .required_content_size_identifiers = identifiers;
}

- (id)currentContentSizeIdentifier {
    env.objc.borrow::<ADBannerViewHostObject>(this).current_content_size_identifier
}

- (())setCurrentContentSizeIdentifier:(id)identifier {
    let old = env.objc.borrow::<ADBannerViewHostObject>(this)
        .current_content_size_identifier;
    release(env, old);
    retain(env, identifier);
    env.objc.borrow_mut::<ADBannerViewHostObject>(this)
        .current_content_size_identifier = identifier;
}

// MARK: - Size helpers

- (CGSize)sizeThatFits:(CGSize)_size {
    // Standard banner size — 320×50 portrait.
    CGSize { width: 320.0, height: 50.0 }
}

// MARK: - Action cancellation

- (())cancelBannerViewAction {
    log!("ADBannerView cancelBannerViewAction: stubbed (no action in progress)");
}

@end

// MARK: - ADInterstitialAd

@implementation ADInterstitialAd: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ADInterstitialAdHostObject {
        delegate: nil,
        loaded: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    // delegate is weak
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)delegate {
    env.objc.borrow::<ADInterstitialAdHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<ADInterstitialAdHostObject>(this).delegate = delegate;
    // Immediately report failure.
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "interstitialAd:didFailWithError:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate interstitialAd:this didFailWithError:nil];
        }
    }
}

- (bool)isLoaded {
    false
}

- (bool)presentFromViewController:(id)_view_controller {
    log!("ADInterstitialAd presentFromViewController: stubbed — ad never shown");
    false
}

- (())presentInView:(id)_view {
    log!("ADInterstitialAd presentInView: stubbed — ad never shown");
}

@end

// MARK: - ADClient (iOS 7+ attribution stub)

@implementation ADClient: NSObject

+ (id)sharedClient {
    nil
}

- (())determineAppInstallationAttributionWithCompletionHandler:(id)_handler {
    log!("ADClient determineAppInstallationAttributionWithCompletionHandler: stubbed");
}

- (())requestAttributionDetailsWithBlock:(id)_block {
    log!("ADClient requestAttributionDetailsWithBlock: stubbed");
}

- (())addClientToSegments:(id)_segment_identifiers // NSArray*
         replaceExisting:(bool)_replace {
    log!("ADClient addClientToSegments:replaceExisting: stubbed");
}

@end

};
