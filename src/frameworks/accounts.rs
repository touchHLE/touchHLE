/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `Accounts.framework` — Apple's iOS-5/6 unified credential store.
//!
//! Apps that link the framework reach the `ACAccountTypeIdentifier*` and
//! `ACFacebook*` (later: `ACTencentWeibo*`, `ACSinaWeibo*`,
//! `ACLinkedIn*`) string constants by Mach-O symbol lookup. Without
//! a [`crate::dyld::HostDylib`] entry the slots remain NULL and any
//! guest-side `[NSString isEqualToString:ACAccountTypeIdentifierFacebook]`
//! dereferences NULL.
//!
//! touchHLE has no real ACAccountStore implementation — the OAuth flow
//! it would drive doesn't exist on Android — but resolving the
//! constants is enough to keep apps that defensively link Accounts (via
//! Facebook SDK / Flurry / Talking Carl's sharing hooks) past the dyld
//! warning and into their normal "account isn't configured" code path.
//!
//! Reference: Apple `Accounts.framework` headers (`ACAccountType.h`,
//! `ACAccountStore.h`, `ACFacebookAccountType.h`).

use crate::dyld::{ConstantExports, FunctionExports, HostConstant, HostDylib};

pub const CONSTANTS: ConstantExports = &[
    // ACAccountType identifiers — apps pass these to
    // -[ACAccountStore accountTypeWithAccountTypeIdentifier:].
    // Literal values come from Apple's headers (com.apple.* reverse
    // DNS namespace).
    (
        "_ACAccountTypeIdentifierTwitter",
        HostConstant::NSString("com.apple.twitter"),
    ),
    (
        "_ACAccountTypeIdentifierFacebook",
        HostConstant::NSString("com.apple.facebook"),
    ),
    (
        "_ACAccountTypeIdentifierSinaWeibo",
        HostConstant::NSString("com.apple.sinaweibo"),
    ),
    (
        "_ACAccountTypeIdentifierTencentWeibo",
        HostConstant::NSString("com.apple.tencentweibo"),
    ),
    (
        "_ACAccountTypeIdentifierLinkedIn",
        HostConstant::NSString("com.apple.linkedin"),
    ),
    // Facebook-specific options dictionary keys used with
    // -[ACAccountStore requestAccessToAccountsWithType:options:completion:].
    (
        "_ACFacebookAppIdKey",
        HostConstant::NSString("ACFacebookAppIdKey"),
    ),
    (
        "_ACFacebookPermissionsKey",
        HostConstant::NSString("ACFacebookPermissionsKey"),
    ),
    (
        "_ACFacebookAudienceKey",
        HostConstant::NSString("ACFacebookAudienceKey"),
    ),
    // Audience-value singletons. Apple ships these as `NSString *
    // const`. Apps compare with `isEqualToString:`; literals match
    // Apple's documented values.
    (
        "_ACFacebookAudienceEveryone",
        HostConstant::NSString("ACFacebookAudienceEveryone"),
    ),
    (
        "_ACFacebookAudienceFriends",
        HostConstant::NSString("ACFacebookAudienceFriends"),
    ),
    (
        "_ACFacebookAudienceOnlyMe",
        HostConstant::NSString("ACFacebookAudienceOnlyMe"),
    ),
    // Tencent / Sina Weibo / LinkedIn options dictionary keys exported
    // alongside Facebook's.
    (
        "_ACTencentWeiboAppIdKey",
        HostConstant::NSString("ACTencentWeiboAppIdKey"),
    ),
    (
        "_ACLinkedInAppIdKey",
        HostConstant::NSString("ACLinkedInAppIdKey"),
    ),
    (
        "_ACLinkedInPermissionsKey",
        HostConstant::NSString("ACLinkedInPermissionsKey"),
    ),
    // Notification name posted when ACAccountStore detects an external
    // change to its database.
    (
        "_ACAccountStoreDidChangeNotification",
        HostConstant::NSString("ACAccountStoreDidChangeNotification"),
    ),
    // Error domain used by all -[ACAccountStore *] failures.
    (
        "_ACErrorDomain",
        HostConstant::NSString("com.apple.accounts"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Accounts.framework/Accounts",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};
