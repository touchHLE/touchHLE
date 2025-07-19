use crate::dyld::{ConstantExports, HostConstant};

pub const kCAFillModeRemoved: &str = "kCAFillModeRemoved";
pub const kCAFillModeForwards: &str = "kCAFillModeForwards";
pub const kCLLocationAccuracyBest: &str = "kCLLocationAccuracyBest";
pub const kCATransitionFromTop: &str = "kCATransitionFromTop";
pub const kCATransitionFromBottom: &str = "kCATransitionFromBottom";
pub const kCFStreamErrorDomainSystemConfiguration: &str = "kCFStreamErrorDomainSystemConfiguration";
pub const kCFStreamPropertySSLSettings: &str = "kCFStreamPropertySSLSettings";
pub const kCFStreamErrorDomainSOCKS: &str = "kCFStreamErrorDomainSOCKS";
pub const kCFStreamPropertyShouldCloseNativeSocket: &str =
    "kCFStreamPropertyShouldCloseNativeSocket";
pub const kCFStreamErrorDomainNetDB: &str = "kCFStreamErrorDomainNetDB";
pub const kCFStreamPropertySocketNativeHandle: &str = "kCFStreamPropertySocketNativeHandle";
pub const kCFStreamErrorDomainMach: &str = "kCFStreamErrorDomainMach";
pub const kCFStreamErrorDomainSSL: &str = "kCFStreamErrorDomainSSL";
pub const kCFStreamErrorDomainNetServices: &str = "kCFStreamErrorDomainNetServices";
pub const kCFStreamSSLAllowsAnyRoot: &str = "kCFStreamSSLAllowsAnyRoot";
pub const kCFStreamSocketSecurityLevelNegotiatedSSL: &str =
    "kCFStreamSocketSecurityLevelNegotiatedSSL";
pub const kCFStreamSSLValidatesCertificateChain: &str = "kCFStreamSSLValidatesCertificateChain";
pub const kCFStreamSSLLevel: &str = "kCFStreamSSLLevel";
pub const kCFTypeArrayCallBacks: &str = "kCFTypeArrayCallBacks";
pub const kSecAttrServer: &str = "kSecAttrServer";
pub const kSecAttrAuthenticationTypeDefault: &str = "kSecAttrAuthenticationTypeDefault";
pub const kSecMatchLimit: &str = "kSecMatchLimit";
pub const kSecClassInternetPassword: &str = "kSecClassInternetPassword";
pub const kSecMatchLimitOne: &str = "kSecMatchLimitOne";
pub const kSecAttrAuthenticationType: &str = "kSecAttrAuthenticationType";
pub const kSecAttrSecurityDomain: &str = "kSecAttrSecurityDomain";
pub const kSecClass: &str = "kSecClass";
pub const kSecAttrType: &str = "kSecAttrType";
pub const kSecValueData: &str = "kSecValueData";
pub const kSecReturnData: &str = "kSecReturnData";
pub const kSecAttrAccount: &str = "kSecAttrAccount";
pub const kABPersonEmailProperty: &str = "kABPersonEmailProperty";
pub const kABPersonFirstNameProperty: &str = "kABPersonFirstNameProperty";
pub const kABPersonPhoneProperty: &str = "kABPersonPhoneProperty";
pub const kABPersonLastNameProperty: &str = "kABPersonLastNameProperty";

pub const NSPOSIXErrorDomain: &str = "NSPOSIXErrorDomain";
pub const NSHTTPCookieName: &str = "NSHTTPCookieName";
pub const NSHTTPCookiePath: &str = "NSHTTPCookiePath";
pub const NSHTTPCookieValue: &str = "NSHTTPCookieValue";
pub const NSHTTPCookieDomain: &str = "NSHTTPCookieDomain";
pub const NSMachErrorDomain: &str = "NSMachErrorDomain";
pub const NSURLErrorDomain: &str = "NSURLErrorDomain";
pub const NSLocaleIdentifier: &str = "NSLocaleIdentifier";

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSPOSIXErrorDomain",
        HostConstant::NSString(NSPOSIXErrorDomain),
    ),
    (
        "_NSHTTPCookieName",
        HostConstant::NSString(NSHTTPCookieName),
    ),
    (
        "_NSHTTPCookiePath",
        HostConstant::NSString(NSHTTPCookiePath),
    ),
    (
        "_NSHTTPCookieValue",
        HostConstant::NSString(NSHTTPCookieValue),
    ),
    (
        "_NSHTTPCookieDomain",
        HostConstant::NSString(NSHTTPCookieDomain),
    ),
    (
        "_NSMachErrorDomain",
        HostConstant::NSString(NSMachErrorDomain),
    ),
    (
        "_NSURLErrorDomain",
        HostConstant::NSString(NSURLErrorDomain),
    ),
    (
        "_NSLocaleIdentifier",
        HostConstant::NSString(NSLocaleIdentifier),
    ),
    (
        "_kCAFillModeRemoved",
        HostConstant::NSString(kCAFillModeRemoved),
    ),
    (
        "_kCAFillModeForwards",
        HostConstant::NSString(kCAFillModeForwards),
    ),
    (
        "_kCLLocationAccuracyBest",
        HostConstant::NSString(kCLLocationAccuracyBest),
    ),
    (
        "_kCATransitionFromTop",
        HostConstant::NSString(kCATransitionFromTop),
    ),
    (
        "_kCATransitionFromBottom",
        HostConstant::NSString(kCATransitionFromBottom),
    ),
    (
        "_kCFStreamErrorDomainSystemConfiguration",
        HostConstant::NSString(kCFStreamErrorDomainSystemConfiguration),
    ),
    (
        "_kCFStreamPropertySSLSettings",
        HostConstant::NSString(kCFStreamPropertySSLSettings),
    ),
    (
        "_kCFStreamErrorDomainSOCKS",
        HostConstant::NSString(kCFStreamErrorDomainSOCKS),
    ),
    (
        "_kCFStreamPropertyShouldCloseNativeSocket",
        HostConstant::NSString(kCFStreamPropertyShouldCloseNativeSocket),
    ),
    (
        "_kCFStreamErrorDomainNetDB",
        HostConstant::NSString(kCFStreamErrorDomainNetDB),
    ),
    (
        "_kCFStreamPropertySocketNativeHandle",
        HostConstant::NSString(kCFStreamPropertySocketNativeHandle),
    ),
    (
        "_kCFStreamErrorDomainMach",
        HostConstant::NSString(kCFStreamErrorDomainMach),
    ),
    (
        "_kCFStreamErrorDomainSSL",
        HostConstant::NSString(kCFStreamErrorDomainSSL),
    ),
    (
        "_kCFStreamErrorDomainNetServices",
        HostConstant::NSString(kCFStreamErrorDomainNetServices),
    ),
    (
        "_kCFStreamSSLAllowsAnyRoot",
        HostConstant::NSString(kCFStreamSSLAllowsAnyRoot),
    ),
    (
        "_kCFStreamSocketSecurityLevelNegotiatedSSL",
        HostConstant::NSString(kCFStreamSocketSecurityLevelNegotiatedSSL),
    ),
    (
        "_kCFStreamSSLValidatesCertificateChain",
        HostConstant::NSString(kCFStreamSSLValidatesCertificateChain),
    ),
    (
        "_kCFStreamSSLLevel",
        HostConstant::NSString(kCFStreamSSLLevel),
    ),
    (
        "_kCFTypeArrayCallBacks",
        HostConstant::NSString(kCFTypeArrayCallBacks),
    ),
    ("_kSecAttrServer", HostConstant::NSString(kSecAttrServer)),
    (
        "_kSecAttrAuthenticationTypeDefault",
        HostConstant::NSString(kSecAttrAuthenticationTypeDefault),
    ),
    ("_kSecMatchLimit", HostConstant::NSString(kSecMatchLimit)),
    (
        "_kSecClassInternetPassword",
        HostConstant::NSString(kSecClassInternetPassword),
    ),
    (
        "_kSecMatchLimitOne",
        HostConstant::NSString(kSecMatchLimitOne),
    ),
    (
        "_kSecAttrAuthenticationType",
        HostConstant::NSString(kSecAttrAuthenticationType),
    ),
    (
        "_kSecAttrSecurityDomain",
        HostConstant::NSString(kSecAttrSecurityDomain),
    ),
    ("_kSecClass", HostConstant::NSString(kSecClass)),
    ("_kSecAttrType", HostConstant::NSString(kSecAttrType)),
    ("_kSecValueData", HostConstant::NSString(kSecValueData)),
    ("_kSecReturnData", HostConstant::NSString(kSecReturnData)),
    ("_kSecAttrAccount", HostConstant::NSString(kSecAttrAccount)),
    (
        "_kABPersonEmailProperty",
        HostConstant::NSString(kABPersonEmailProperty),
    ),
    (
        "_kABPersonFirstNameProperty",
        HostConstant::NSString(kABPersonFirstNameProperty),
    ),
    (
        "_kABPersonPhoneProperty",
        HostConstant::NSString(kABPersonPhoneProperty),
    ),
    (
        "_kABPersonLastNameProperty",
        HostConstant::NSString(kABPersonLastNameProperty),
    ),
];
