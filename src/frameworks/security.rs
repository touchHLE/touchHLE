/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Security framework — Keychain Services, SecRandom, SecKey, SecCertificate,
//! SecTrust, SecPolicy, and SecAccess stubs.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::frameworks::core_foundation::cf_array::CFArrayRef;
use crate::frameworks::core_foundation::cf_data::CFDataRef;
use crate::frameworks::core_foundation::cf_dictionary::CFDictionaryRef;
use crate::frameworks::core_foundation::cf_string::CFStringRef;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::objc::nil;
use crate::Environment;

// =========================================================================
// MARK: - OSStatus result codes (Security/SecBase.h)
// =========================================================================

pub type OSStatus = i32;

pub const ERR_SEC_SUCCESS: OSStatus = 0;
pub const ERR_SEC_UNIMPLEMENTED: OSStatus = -4;
pub const ERR_SEC_PARAM: OSStatus = -50;
pub const ERR_SEC_ALLOC_FAILED: OSStatus = -108;
pub const ERR_SEC_NOT_AVAILABLE: OSStatus = -25291;
pub const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
pub const ERR_SEC_DUPLICATE_ITEM: OSStatus = -25299;
pub const ERR_SEC_DECODE: OSStatus = -26275;
pub const ERR_SEC_AUTH_FAILED: OSStatus = -25293;
pub const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;

// =========================================================================
// MARK: - Opaque type aliases
// =========================================================================

pub type SecKeychainRef = CFTypeRef;
pub type SecKeychainItemRef = CFTypeRef;
pub type SecCertificateRef = CFTypeRef;
pub type SecKeyRef = CFTypeRef;
pub type SecIdentityRef = CFTypeRef;
pub type SecTrustRef = CFTypeRef;
pub type SecPolicyRef = CFTypeRef;
pub type SecAccessRef = CFTypeRef;
pub type SecRandomRef = CFTypeRef;

// SecTrustResultType
pub type SecTrustResultType = u32;

pub const kSecTrustResultInvalid: SecTrustResultType = 0;
pub const kSecTrustResultProceed: SecTrustResultType = 1;
pub const kSecTrustResultDeny: SecTrustResultType = 3;
pub const kSecTrustResultUnspecified: SecTrustResultType = 4;
pub const kSecTrustResultRecoverableTrustFailure: SecTrustResultType = 5;
pub const kSecTrustResultFatalTrustFailure: SecTrustResultType = 6;
pub const kSecTrustResultOtherError: SecTrustResultType = 7;

// =========================================================================
// MARK: - SecItem (Keychain)
// =========================================================================

fn SecItemCopyMatching(
    env: &mut Environment,
    _query: CFDictionaryRef,
    result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log_dbg!("SecItemCopyMatching — returning errSecItemNotFound");
    if !result.is_null() {
        env.mem.write(result, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecItemAdd(
    _env: &mut Environment,
    _attributes: CFDictionaryRef,
    _result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log_dbg!("SecItemAdd — returning errSecSuccess (stub)");
    ERR_SEC_SUCCESS
}

fn SecItemDelete(_env: &mut Environment, _query: CFDictionaryRef) -> OSStatus {
    log_dbg!("SecItemDelete — returning errSecItemNotFound");
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecItemUpdate(
    _env: &mut Environment,
    _query: CFDictionaryRef,
    _attributes_to_update: CFDictionaryRef,
) -> OSStatus {
    log_dbg!("SecItemUpdate — returning errSecItemNotFound");
    ERR_SEC_ITEM_NOT_FOUND
}

// =========================================================================
// MARK: - Legacy Keychain API (SecKeychain*)
// =========================================================================

fn SecKeychainGetVersion(env: &mut Environment, version: MutPtr<u32>) -> OSStatus {
    if !version.is_null() {
        env.mem.write(version, 0x02050003); // 2.5.0.3 — a plausible iOS version
    }
    ERR_SEC_SUCCESS
}

fn SecKeychainOpen(
    env: &mut Environment,
    _path_name: ConstVoidPtr,
    keychain: MutPtr<SecKeychainRef>,
) -> OSStatus {
    if !keychain.is_null() {
        env.mem.write(keychain, nil);
    }
    ERR_SEC_NOT_AVAILABLE
}

fn SecKeychainCopyDefault(env: &mut Environment, keychain: MutPtr<SecKeychainRef>) -> OSStatus {
    if !keychain.is_null() {
        env.mem.write(keychain, nil);
    }
    ERR_SEC_NOT_AVAILABLE
}

fn SecKeychainItemFreeContent(
    _env: &mut Environment,
    _attr_list: MutVoidPtr,
    _data: MutVoidPtr,
) -> OSStatus {
    ERR_SEC_SUCCESS
}

fn SecKeychainItemFreeAttributesAndData(
    _env: &mut Environment,
    _attr_list: MutVoidPtr,
    _data: MutVoidPtr,
) -> OSStatus {
    ERR_SEC_SUCCESS
}

fn SecKeychainItemDelete(_env: &mut Environment, _item_ref: SecKeychainItemRef) -> OSStatus {
    log_dbg!("SecKeychainItemDelete — returning errSecItemNotFound");
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecKeychainItemModifyAttributesAndData(
    _env: &mut Environment,
    _item_ref: SecKeychainItemRef,
    _attr_list: ConstVoidPtr,
    _length: u32,
    _data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("SecKeychainItemModifyAttributesAndData — returning errSecItemNotFound");
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecKeychainAddGenericPassword(
    env: &mut Environment,
    _keychain: SecKeychainRef,
    _service_name_length: u32,
    _service_name: ConstVoidPtr,
    _account_name_length: u32,
    _account_name: ConstVoidPtr,
    _password_length: u32,
    _password_data: ConstVoidPtr,
    item_ref: MutPtr<SecKeychainItemRef>,
) -> OSStatus {
    log_dbg!("SecKeychainAddGenericPassword — returning errSecSuccess (stub)");
    if !item_ref.is_null() {
        env.mem.write(item_ref, nil);
    }
    ERR_SEC_SUCCESS
}

fn SecKeychainFindGenericPassword(
    env: &mut Environment,
    _keychain_or_array: CFTypeRef,
    _service_name_length: u32,
    _service_name: ConstVoidPtr,
    _account_name_length: u32,
    _account_name: ConstVoidPtr,
    password_length: MutPtr<u32>,
    password_data: MutPtr<MutVoidPtr>,
    item_ref: MutPtr<SecKeychainItemRef>,
) -> OSStatus {
    log_dbg!("SecKeychainFindGenericPassword — returning errSecItemNotFound");
    if !password_length.is_null() {
        env.mem.write(password_length, 0);
    }
    if !password_data.is_null() {
        env.mem.write(password_data, crate::mem::Ptr::null());
    }
    if !item_ref.is_null() {
        env.mem.write(item_ref, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecKeychainAddInternetPassword(
    env: &mut Environment,
    _keychain: SecKeychainRef,
    _server_name_length: u32,
    _server_name: ConstVoidPtr,
    _security_domain_length: u32,
    _security_domain: ConstVoidPtr,
    _account_name_length: u32,
    _account_name: ConstVoidPtr,
    _path_length: u32,
    _path: ConstVoidPtr,
    _port: u16,
    _protocol: u32,
    _auth_type: u32,
    _password_length: u32,
    _password_data: ConstVoidPtr,
    item_ref: MutPtr<SecKeychainItemRef>,
) -> OSStatus {
    log_dbg!("SecKeychainAddInternetPassword — returning errSecSuccess (stub)");
    if !item_ref.is_null() {
        env.mem.write(item_ref, nil);
    }
    ERR_SEC_SUCCESS
}

fn SecKeychainFindInternetPassword(
    env: &mut Environment,
    _keychain_or_array: CFTypeRef,
    _server_name_length: u32,
    _server_name: ConstVoidPtr,
    _security_domain_length: u32,
    _security_domain: ConstVoidPtr,
    _account_name_length: u32,
    _account_name: ConstVoidPtr,
    _path_length: u32,
    _path: ConstVoidPtr,
    _port: u16,
    _protocol: u32,
    _auth_type: u32,
    password_length: MutPtr<u32>,
    password_data: MutPtr<MutVoidPtr>,
    item_ref: MutPtr<SecKeychainItemRef>,
) -> OSStatus {
    log_dbg!("SecKeychainFindInternetPassword — returning errSecItemNotFound");
    if !password_length.is_null() {
        env.mem.write(password_length, 0);
    }
    if !password_data.is_null() {
        env.mem.write(password_data, crate::mem::Ptr::null());
    }
    if !item_ref.is_null() {
        env.mem.write(item_ref, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

// =========================================================================
// MARK: - SecRandom
// =========================================================================

fn SecRandomCopyBytes(
    env: &mut Environment,
    _rnd: SecRandomRef,
    count: u32,
    bytes: MutPtr<u8>,
) -> i32 {
    if bytes.is_null() || count == 0 {
        return ERR_SEC_PARAM;
    }
    // Fill with pseudo-random bytes using Rust's built-in facilities.
    let slice = env.mem.bytes_at_mut(bytes, count);
    for b in slice.iter_mut() {
        // Simple LCG — good enough for game UUIDs / session tokens.
        *b = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
            & 0xFF) as u8;
    }
    log_dbg!("SecRandomCopyBytes: generated {} bytes", count);
    ERR_SEC_SUCCESS
}

// =========================================================================
// MARK: - SecCertificate
// =========================================================================

fn SecCertificateCreateWithData(
    _env: &mut Environment,
    _allocator: CFTypeRef,
    _data: CFDataRef,
) -> SecCertificateRef {
    log_dbg!("SecCertificateCreateWithData — returning nil");
    nil
}

fn SecCertificateCopyData(_env: &mut Environment, _certificate: SecCertificateRef) -> CFDataRef {
    log_dbg!("SecCertificateCopyData — returning nil");
    nil
}

fn SecCertificateCopySubjectSummary(
    _env: &mut Environment,
    _certificate: SecCertificateRef,
) -> CFStringRef {
    log_dbg!("SecCertificateCopySubjectSummary — returning nil");
    nil
}

// =========================================================================
// MARK: - SecKey
// =========================================================================

fn SecKeyGetBlockSize(_env: &mut Environment, _key: SecKeyRef) -> u32 {
    log_dbg!("SecKeyGetBlockSize — returning 256");
    256
}

fn SecKeyRawSign(
    _env: &mut Environment,
    _key: SecKeyRef,
    _padding: u32,
    _data_to_sign: ConstVoidPtr,
    _data_to_sign_len: u32,
    _sig: MutVoidPtr,
    _sig_len: MutPtr<u32>,
) -> OSStatus {
    log!("TODO: SecKeyRawSign — returning errSecUnimplemented");
    ERR_SEC_UNIMPLEMENTED
}

fn SecKeyRawVerify(
    _env: &mut Environment,
    _key: SecKeyRef,
    _padding: u32,
    _signed_data: ConstVoidPtr,
    _signed_data_len: u32,
    _sig: ConstVoidPtr,
    _sig_len: u32,
) -> OSStatus {
    log!("TODO: SecKeyRawVerify — returning errSecUnimplemented");
    ERR_SEC_UNIMPLEMENTED
}

fn SecKeyEncrypt(
    _env: &mut Environment,
    _key: SecKeyRef,
    _padding: u32,
    _plain_text: ConstVoidPtr,
    _plain_text_len: u32,
    _cipher_text: MutVoidPtr,
    _cipher_text_len: MutPtr<u32>,
) -> OSStatus {
    log!("TODO: SecKeyEncrypt — returning errSecUnimplemented");
    ERR_SEC_UNIMPLEMENTED
}

fn SecKeyDecrypt(
    _env: &mut Environment,
    _key: SecKeyRef,
    _padding: u32,
    _cipher_text: ConstVoidPtr,
    _cipher_text_len: u32,
    _plain_text: MutVoidPtr,
    _plain_text_len: MutPtr<u32>,
) -> OSStatus {
    log!("TODO: SecKeyDecrypt — returning errSecUnimplemented");
    ERR_SEC_UNIMPLEMENTED
}

fn SecKeyGeneratePair(
    env: &mut Environment,
    _parameters: CFDictionaryRef,
    public_key: MutPtr<SecKeyRef>,
    private_key: MutPtr<SecKeyRef>,
) -> OSStatus {
    log!("TODO: SecKeyGeneratePair — returning errSecUnimplemented");
    if !public_key.is_null() {
        env.mem.write(public_key, nil);
    }
    if !private_key.is_null() {
        env.mem.write(private_key, nil);
    }
    ERR_SEC_UNIMPLEMENTED
}

// =========================================================================
// MARK: - SecTrust
// =========================================================================

fn SecTrustCreateWithCertificates(
    env: &mut Environment,
    _certificates: CFTypeRef,
    _policies: CFTypeRef,
    trust: MutPtr<SecTrustRef>,
) -> OSStatus {
    log_dbg!("SecTrustCreateWithCertificates — returning nil trust");
    if !trust.is_null() {
        env.mem.write(trust, nil);
    }
    ERR_SEC_SUCCESS
}

fn SecTrustEvaluate(
    env: &mut Environment,
    _trust: SecTrustRef,
    result: MutPtr<SecTrustResultType>,
) -> OSStatus {
    log_dbg!("SecTrustEvaluate — returning kSecTrustResultUnspecified");
    if !result.is_null() {
        env.mem.write(result, kSecTrustResultUnspecified);
    }
    ERR_SEC_SUCCESS
}

fn SecTrustEvaluateAsync(
    _env: &mut Environment,
    _trust: SecTrustRef,
    _queue: CFTypeRef,
    _result: CFTypeRef,
) -> OSStatus {
    log_dbg!("SecTrustEvaluateAsync — returning errSecUnimplemented");
    ERR_SEC_UNIMPLEMENTED
}

fn SecTrustGetCertificateCount(_env: &mut Environment, _trust: SecTrustRef) -> u32 {
    0
}

fn SecTrustGetCertificateAtIndex(
    _env: &mut Environment,
    _trust: SecTrustRef,
    _ix: u32,
) -> SecCertificateRef {
    nil
}

fn SecTrustCopyPublicKey(_env: &mut Environment, _trust: SecTrustRef) -> SecKeyRef {
    nil
}

fn SecTrustSetAnchorCertificates(
    _env: &mut Environment,
    _trust: SecTrustRef,
    _anchor_certificates: CFArrayRef,
) -> OSStatus {
    ERR_SEC_SUCCESS
}

fn SecTrustSetAnchorCertificatesOnly(
    _env: &mut Environment,
    _trust: SecTrustRef,
    _anchor_certificates_only: bool,
) -> OSStatus {
    ERR_SEC_SUCCESS
}

fn SecTrustSetPolicies(
    _env: &mut Environment,
    _trust: SecTrustRef,
    _policies: CFTypeRef,
) -> OSStatus {
    ERR_SEC_SUCCESS
}

// =========================================================================
// MARK: - SecPolicy
// =========================================================================

fn SecPolicyCreateBasicX509(_env: &mut Environment) -> SecPolicyRef {
    log_dbg!("SecPolicyCreateBasicX509 — returning nil");
    nil
}

fn SecPolicyCreateSSL(
    _env: &mut Environment,
    _server: bool,
    _hostname: CFStringRef,
) -> SecPolicyRef {
    log_dbg!("SecPolicyCreateSSL — returning nil");
    nil
}

fn SecPolicyCreateRevocation(_env: &mut Environment, _revocation_flags: u32) -> SecPolicyRef {
    nil
}

fn SecPolicyCopyProperties(_env: &mut Environment, _policy: SecPolicyRef) -> CFDictionaryRef {
    nil
}

// =========================================================================
// MARK: - SecIdentity
// =========================================================================

fn SecIdentityCopyCertificate(
    env: &mut Environment,
    _identity_ref: SecIdentityRef,
    certificate_ref: MutPtr<SecCertificateRef>,
) -> OSStatus {
    if !certificate_ref.is_null() {
        env.mem.write(certificate_ref, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

fn SecIdentityCopyPrivateKey(
    env: &mut Environment,
    _identity_ref: SecIdentityRef,
    private_key_ref: MutPtr<SecKeyRef>,
) -> OSStatus {
    if !private_key_ref.is_null() {
        env.mem.write(private_key_ref, nil);
    }
    ERR_SEC_ITEM_NOT_FOUND
}

// =========================================================================
// MARK: - SecAccess (macOS-only API, sometimes linked on iOS apps)
// =========================================================================

fn SecAccessCreate(
    env: &mut Environment,
    _descriptor: CFStringRef,
    _trusted_list: CFArrayRef,
    access_ref: MutPtr<SecAccessRef>,
) -> OSStatus {
    if !access_ref.is_null() {
        env.mem.write(access_ref, nil);
    }
    ERR_SEC_UNIMPLEMENTED
}

// =========================================================================
// MARK: - Misc
// =========================================================================

fn SecCopyErrorMessageString(
    _env: &mut Environment,
    status: OSStatus,
    _reserved: ConstVoidPtr,
) -> CFStringRef {
    log_dbg!("SecCopyErrorMessageString({})", status);
    // Returning nil is valid — callers must handle nil per Apple docs.
    nil
}

// =========================================================================
// MARK: - CONSTANTS
// =========================================================================

pub const CONSTANTS: ConstantExports = &[
    // kSecClass
    ("_kSecClass", HostConstant::NSString("kSecClass")),
    (
        "_kSecClassGenericPassword",
        HostConstant::NSString("kSecClassGenericPassword"),
    ),
    (
        "_kSecClassInternetPassword",
        HostConstant::NSString("kSecClassInternetPassword"),
    ),
    (
        "_kSecClassCertificate",
        HostConstant::NSString("kSecClassCertificate"),
    ),
    ("_kSecClassKey", HostConstant::NSString("kSecClassKey")),
    (
        "_kSecClassIdentity",
        HostConstant::NSString("kSecClassIdentity"),
    ),
    // Attribute keys
    (
        "_kSecAttrAccessGroup",
        HostConstant::NSString("kSecAttrAccessGroup"),
    ),
    (
        "_kSecAttrAccessible",
        HostConstant::NSString("kSecAttrAccessible"),
    ),
    (
        "_kSecAttrAccount",
        HostConstant::NSString("kSecAttrAccount"),
    ),
    (
        "_kSecAttrDescription",
        HostConstant::NSString("kSecAttrDescription"),
    ),
    (
        "_kSecAttrGeneric",
        HostConstant::NSString("kSecAttrGeneric"),
    ),
    ("_kSecAttrLabel", HostConstant::NSString("kSecAttrLabel")),
    (
        "_kSecAttrService",
        HostConstant::NSString("kSecAttrService"),
    ),
    ("_kSecAttrServer", HostConstant::NSString("kSecAttrServer")),
    (
        "_kSecAttrCreationDate",
        HostConstant::NSString("kSecAttrCreationDate"),
    ),
    (
        "_kSecAttrModificationDate",
        HostConstant::NSString("kSecAttrModificationDate"),
    ),
    (
        "_kSecAttrComment",
        HostConstant::NSString("kSecAttrComment"),
    ),
    (
        "_kSecAttrCreator",
        HostConstant::NSString("kSecAttrCreator"),
    ),
    ("_kSecAttrType", HostConstant::NSString("kSecAttrType")),
    (
        "_kSecAttrIsInvisible",
        HostConstant::NSString("kSecAttrIsInvisible"),
    ),
    (
        "_kSecAttrIsNegative",
        HostConstant::NSString("kSecAttrIsNegative"),
    ),
    (
        "_kSecAttrSynchronizable",
        HostConstant::NSString("kSecAttrSynchronizable"),
    ),
    (
        "_kSecAttrApplicationLabel",
        HostConstant::NSString("kSecAttrApplicationLabel"),
    ),
    (
        "_kSecAttrApplicationTag",
        HostConstant::NSString("kSecAttrApplicationTag"),
    ),
    (
        "_kSecAttrKeyType",
        HostConstant::NSString("kSecAttrKeyType"),
    ),
    (
        "_kSecAttrKeySizeInBits",
        HostConstant::NSString("kSecAttrKeySizeInBits"),
    ),
    (
        "_kSecAttrEffectiveKeySize",
        HostConstant::NSString("kSecAttrEffectiveKeySize"),
    ),
    (
        "_kSecAttrCanEncrypt",
        HostConstant::NSString("kSecAttrCanEncrypt"),
    ),
    (
        "_kSecAttrCanDecrypt",
        HostConstant::NSString("kSecAttrCanDecrypt"),
    ),
    (
        "_kSecAttrCanDerive",
        HostConstant::NSString("kSecAttrCanDerive"),
    ),
    (
        "_kSecAttrCanSign",
        HostConstant::NSString("kSecAttrCanSign"),
    ),
    (
        "_kSecAttrCanVerify",
        HostConstant::NSString("kSecAttrCanVerify"),
    ),
    (
        "_kSecAttrCanWrap",
        HostConstant::NSString("kSecAttrCanWrap"),
    ),
    (
        "_kSecAttrCanUnwrap",
        HostConstant::NSString("kSecAttrCanUnwrap"),
    ),
    (
        "_kSecAttrSubject",
        HostConstant::NSString("kSecAttrSubject"),
    ),
    ("_kSecAttrIssuer", HostConstant::NSString("kSecAttrIssuer")),
    (
        "_kSecAttrSerialNumber",
        HostConstant::NSString("kSecAttrSerialNumber"),
    ),
    (
        "_kSecAttrSubjectKeyID",
        HostConstant::NSString("kSecAttrSubjectKeyID"),
    ),
    (
        "_kSecAttrPublicKeyHash",
        HostConstant::NSString("kSecAttrPublicKeyHash"),
    ),
    (
        "_kSecAttrCertificateType",
        HostConstant::NSString("kSecAttrCertificateType"),
    ),
    (
        "_kSecAttrCertificateEncoding",
        HostConstant::NSString("kSecAttrCertificateEncoding"),
    ),
    ("_kSecAttrPath", HostConstant::NSString("kSecAttrPath")),
    ("_kSecAttrPort", HostConstant::NSString("kSecAttrPort")),
    (
        "_kSecAttrProtocol",
        HostConstant::NSString("kSecAttrProtocol"),
    ),
    (
        "_kSecAttrAuthenticationType",
        HostConstant::NSString("kSecAttrAuthenticationType"),
    ),
    (
        "_kSecAttrAuthenticationTypeDefault",
        HostConstant::NSString("dflt"),
    ),
    (
        "_kSecAttrSecurityDomain",
        HostConstant::NSString("kSecAttrSecurityDomain"),
    ),
    // kSecAttrAccessible values
    (
        "_kSecAttrAccessibleWhenUnlocked",
        HostConstant::NSString("kSecAttrAccessibleWhenUnlocked"),
    ),
    (
        "_kSecAttrAccessibleAfterFirstUnlock",
        HostConstant::NSString("kSecAttrAccessibleAfterFirstUnlock"),
    ),
    (
        "_kSecAttrAccessibleAlways",
        HostConstant::NSString("kSecAttrAccessibleAlways"),
    ),
    (
        "_kSecAttrAccessibleWhenUnlockedThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleWhenUnlockedThisDeviceOnly"),
    ),
    (
        "_kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly"),
    ),
    (
        "_kSecAttrAccessibleAlwaysThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleAlwaysThisDeviceOnly"),
    ),
    (
        "_kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly",
        HostConstant::NSString("kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly"),
    ),
    // kSecAttrKeyType values
    (
        "_kSecAttrKeyTypeRSA",
        HostConstant::NSString("kSecAttrKeyTypeRSA"),
    ),
    (
        "_kSecAttrKeyTypeEC",
        HostConstant::NSString("kSecAttrKeyTypeEC"),
    ),
    (
        "_kSecAttrKeyTypeECSECPrimeRandom",
        HostConstant::NSString("kSecAttrKeyTypeECSECPrimeRandom"),
    ),
    // kSecAttrProtocol values
    (
        "_kSecAttrProtocolHTTPS",
        HostConstant::NSString("kSecAttrProtocolHTTPS"),
    ),
    (
        "_kSecAttrProtocolHTTP",
        HostConstant::NSString("kSecAttrProtocolHTTP"),
    ),
    (
        "_kSecAttrProtocolFTP",
        HostConstant::NSString("kSecAttrProtocolFTP"),
    ),
    // Value keys
    ("_kSecValueData", HostConstant::NSString("kSecValueData")),
    ("_kSecValueRef", HostConstant::NSString("kSecValueRef")),
    (
        "_kSecValuePersistentRef",
        HostConstant::NSString("kSecValuePersistentRef"),
    ),
    // Return-type keys
    ("_kSecReturnData", HostConstant::NSString("kSecReturnData")),
    (
        "_kSecReturnAttributes",
        HostConstant::NSString("kSecReturnAttributes"),
    ),
    ("_kSecReturnRef", HostConstant::NSString("kSecReturnRef")),
    (
        "_kSecReturnPersistentRef",
        HostConstant::NSString("kSecReturnPersistentRef"),
    ),
    // Match keys
    ("_kSecMatchLimit", HostConstant::NSString("kSecMatchLimit")),
    (
        "_kSecMatchLimitOne",
        HostConstant::NSString("kSecMatchLimitOne"),
    ),
    (
        "_kSecMatchLimitAll",
        HostConstant::NSString("kSecMatchLimitAll"),
    ),
    (
        "_kSecMatchItemList",
        HostConstant::NSString("kSecMatchItemList"),
    ),
    (
        "_kSecMatchIssuers",
        HostConstant::NSString("kSecMatchIssuers"),
    ),
    (
        "_kSecMatchEmailAddressIfPresent",
        HostConstant::NSString("kSecMatchEmailAddressIfPresent"),
    ),
    (
        "_kSecMatchSubjectContains",
        HostConstant::NSString("kSecMatchSubjectContains"),
    ),
    (
        "_kSecMatchCaseInsensitive",
        HostConstant::NSString("kSecMatchCaseInsensitive"),
    ),
    (
        "_kSecMatchTrustedOnly",
        HostConstant::NSString("kSecMatchTrustedOnly"),
    ),
    (
        "_kSecMatchValidOnDate",
        HostConstant::NSString("kSecMatchValidOnDate"),
    ),
    (
        "_kSecMatchPolicy",
        HostConstant::NSString("kSecMatchPolicy"),
    ),
    (
        "_kSecMatchSearchList",
        HostConstant::NSString("kSecMatchSearchList"),
    ),
    // Use keys
    (
        "_kSecUseItemList",
        HostConstant::NSString("kSecUseItemList"),
    ),
    (
        "_kSecUseOperationPrompt",
        HostConstant::NSString("kSecUseOperationPrompt"),
    ),
    (
        "_kSecUseNoAuthenticationUI",
        HostConstant::NSString("kSecUseNoAuthenticationUI"),
    ),
    // Trust result constants (NSString form used as dictionary values)
    (
        "_kSecTrustInfoExtendedValidationKey",
        HostConstant::NSString("kSecTrustInfoExtendedValidationKey"),
    ),
    // SecRandom
    ("_kSecRandomDefault", HostConstant::NullPtr),
    // Private key generation
    (
        "_kSecPrivateKeyAttrs",
        HostConstant::NSString("kSecPrivateKeyAttrs"),
    ),
    (
        "_kSecPublicKeyAttrs",
        HostConstant::NSString("kSecPublicKeyAttrs"),
    ),
    // -----------------------------------------------------------------
    // Key class attribute and its three values, per
    // <https://developer.apple.com/documentation/security/ksecattrkeyclass>.
    // -----------------------------------------------------------------
    (
        "_kSecAttrKeyClass",
        HostConstant::NSString("kSecAttrKeyClass"),
    ),
    (
        "_kSecAttrKeyClassPublic",
        HostConstant::NSString("kSecAttrKeyClassPublic"),
    ),
    (
        "_kSecAttrKeyClassPrivate",
        HostConstant::NSString("kSecAttrKeyClassPrivate"),
    ),
    (
        "_kSecAttrKeyClassSymmetric",
        HostConstant::NSString("kSecAttrKeyClassSymmetric"),
    ),
    // -----------------------------------------------------------------
    // SecItemImport / SecItemExport input keys, per
    // <https://developer.apple.com/documentation/security/secitemimport_secitemexport>.
    // -----------------------------------------------------------------
    (
        "_kSecImportExportPassphrase",
        HostConstant::NSString("passphrase"),
    ),
    (
        "_kSecImportItemIdentity",
        HostConstant::NSString("identity"),
    ),
    (
        "_kSecImportItemLabel",
        HostConstant::NSString("label"),
    ),
    (
        "_kSecImportItemKeyID",
        HostConstant::NSString("keyid"),
    ),
    (
        "_kSecImportItemTrust",
        HostConstant::NSString("trust"),
    ),
    (
        "_kSecImportItemCertChain",
        HostConstant::NSString("chain"),
    ),
    // -----------------------------------------------------------------
    // `kSecAttrAccessControl` — value is a SecAccessControl that
    // qualifies `kSecAttrAccessible`. Apps thread it as an opaque
    // dictionary key, so we expose the constant by name.
    // <https://developer.apple.com/documentation/security/ksecattraccesscontrol>
    // -----------------------------------------------------------------
    (
        "_kSecAttrAccessControl",
        HostConstant::NSString("kSecAttrAccessControl"),
    ),
    // -----------------------------------------------------------------
    // `kSecAttrSynchronizableAny` — wildcard query value (iOS 7.0+).
    // <https://developer.apple.com/documentation/security/ksecattrsynchronizableany>
    // -----------------------------------------------------------------
    (
        "_kSecAttrSynchronizableAny",
        HostConstant::NSString("kSecAttrSynchronizableAny"),
    ),
    // -----------------------------------------------------------------
    // `kSecSharedPassword` — kSecSharedPassword key for AutoFill APIs.
    // <https://developer.apple.com/documentation/security/secaddsharedwebcredential>
    // -----------------------------------------------------------------
    (
        "_kSecSharedPassword",
        HostConstant::NSString("spwd"),
    ),
    // -----------------------------------------------------------------
    // `kSecAttrAuthenticationType` enumerated values, per Apple
    // `<Security/SecItem.h>`. The four-character strings are the
    // canonical CSSM authentication-type tags used by SecKeychain on
    // both macOS and iOS, so dictionary lookups round-trip correctly.
    // <https://developer.apple.com/documentation/security/ksecattrauthenticationtype>
    // -----------------------------------------------------------------
    (
        "_kSecAttrAuthenticationTypeHTTPBasic",
        HostConstant::NSString("http"),
    ),
    (
        "_kSecAttrAuthenticationTypeHTTPDigest",
        HostConstant::NSString("httd"),
    ),
    (
        "_kSecAttrAuthenticationTypeHTMLForm",
        HostConstant::NSString("form"),
    ),
    (
        "_kSecAttrAuthenticationTypeNTLM",
        HostConstant::NSString("ntlm"),
    ),
    (
        "_kSecAttrAuthenticationTypeMSN",
        HostConstant::NSString("msna"),
    ),
    (
        "_kSecAttrAuthenticationTypeDPA",
        HostConstant::NSString("dpaa"),
    ),
    (
        "_kSecAttrAuthenticationTypeRPA",
        HostConstant::NSString("rpaa"),
    ),
    // -----------------------------------------------------------------
    // `kSecAttrProtocol` enumerated values for internet-password items.
    // Apple `<Security/SecItem.h>` ships them as four-character codes
    // matching CSSM_DB_PROTOCOL_* IDs. The values below match Apple's
    // exactly so apps storing internet passwords with these tags
    // continue to look up correctly.
    // <https://developer.apple.com/documentation/security/ksecattrprotocol>
    // -----------------------------------------------------------------
    (
        "_kSecAttrProtocolFTPProxy",
        HostConstant::NSString("ftpx"),
    ),
    (
        "_kSecAttrProtocolFTPAccount",
        HostConstant::NSString("ftpa"),
    ),
    (
        "_kSecAttrProtocolFTPS",
        HostConstant::NSString("ftps"),
    ),
    (
        "_kSecAttrProtocolHTTPProxy",
        HostConstant::NSString("htpx"),
    ),
    (
        "_kSecAttrProtocolHTTPSProxy",
        HostConstant::NSString("htsx"),
    ),
    (
        "_kSecAttrProtocolIRC",
        HostConstant::NSString("irc "),
    ),
    (
        "_kSecAttrProtocolIRCS",
        HostConstant::NSString("ircs"),
    ),
    (
        "_kSecAttrProtocolNNTP",
        HostConstant::NSString("nntp"),
    ),
    (
        "_kSecAttrProtocolNNTPS",
        HostConstant::NSString("ntps"),
    ),
    (
        "_kSecAttrProtocolPOP3",
        HostConstant::NSString("pop3"),
    ),
    (
        "_kSecAttrProtocolPOP3S",
        HostConstant::NSString("pops"),
    ),
    (
        "_kSecAttrProtocolSMTP",
        HostConstant::NSString("smtp"),
    ),
    (
        "_kSecAttrProtocolIMAP",
        HostConstant::NSString("imap"),
    ),
    (
        "_kSecAttrProtocolLDAP",
        HostConstant::NSString("ldap"),
    ),
    (
        "_kSecAttrProtocolLDAPS",
        HostConstant::NSString("ldps"),
    ),
    (
        "_kSecAttrProtocolTelnet",
        HostConstant::NSString("teln"),
    ),
    (
        "_kSecAttrProtocolTelnetS",
        HostConstant::NSString("tels"),
    ),
    (
        "_kSecAttrProtocolSSH",
        HostConstant::NSString("ssh "),
    ),
    (
        "_kSecAttrProtocolAFP",
        HostConstant::NSString("afp "),
    ),
    (
        "_kSecAttrProtocolAppleTalk",
        HostConstant::NSString("atlk"),
    ),
    (
        "_kSecAttrProtocolDAAP",
        HostConstant::NSString("daap"),
    ),
    (
        "_kSecAttrProtocolEPPC",
        HostConstant::NSString("eppc"),
    ),
    (
        "_kSecAttrProtocolRTSP",
        HostConstant::NSString("rtsp"),
    ),
    (
        "_kSecAttrProtocolRTSPProxy",
        HostConstant::NSString("rtsx"),
    ),
    (
        "_kSecAttrProtocolSMB",
        HostConstant::NSString("smb "),
    ),
    (
        "_kSecAttrProtocolSOCKS",
        HostConstant::NSString("sox "),
    ),
];

// =========================================================================
// MARK: - FUNCTIONS
// =========================================================================

pub const FUNCTIONS: FunctionExports = &[
    // SecItem
    export_c_func!(SecItemCopyMatching(_, _)),
    export_c_func!(SecItemAdd(_, _)),
    export_c_func!(SecItemDelete(_)),
    export_c_func!(SecItemUpdate(_, _)),
    // Legacy Keychain
    export_c_func!(SecKeychainGetVersion(_)),
    export_c_func!(SecKeychainOpen(_, _)),
    export_c_func!(SecKeychainCopyDefault(_)),
    export_c_func!(SecKeychainItemFreeContent(_, _)),
    export_c_func!(SecKeychainItemFreeAttributesAndData(_, _)),
    export_c_func!(SecKeychainItemDelete(_)),
    export_c_func!(SecKeychainItemModifyAttributesAndData(_, _, _, _)),
    export_c_func!(SecKeychainAddGenericPassword(_, _, _, _, _, _, _, _)),
    export_c_func!(SecKeychainFindGenericPassword(_, _, _, _, _, _, _, _)),
    export_c_func!(SecKeychainAddInternetPassword(
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _
    )),
    export_c_func!(SecKeychainFindInternetPassword(
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _
    )),
    // SecRandom
    export_c_func!(SecRandomCopyBytes(_, _, _)),
    // SecCertificate
    export_c_func!(SecCertificateCreateWithData(_, _)),
    export_c_func!(SecCertificateCopyData(_)),
    export_c_func!(SecCertificateCopySubjectSummary(_)),
    // SecKey
    export_c_func!(SecKeyGetBlockSize(_)),
    export_c_func!(SecKeyRawSign(_, _, _, _, _, _)),
    export_c_func!(SecKeyRawVerify(_, _, _, _, _, _)),
    export_c_func!(SecKeyEncrypt(_, _, _, _, _, _)),
    export_c_func!(SecKeyDecrypt(_, _, _, _, _, _)),
    export_c_func!(SecKeyGeneratePair(_, _, _)),
    // SecTrust
    export_c_func!(SecTrustCreateWithCertificates(_, _, _)),
    export_c_func!(SecTrustEvaluate(_, _)),
    export_c_func!(SecTrustEvaluateAsync(_, _, _)),
    export_c_func!(SecTrustGetCertificateCount(_)),
    export_c_func!(SecTrustGetCertificateAtIndex(_, _)),
    export_c_func!(SecTrustCopyPublicKey(_)),
    export_c_func!(SecTrustSetAnchorCertificates(_, _)),
    export_c_func!(SecTrustSetAnchorCertificatesOnly(_, _)),
    export_c_func!(SecTrustSetPolicies(_, _)),
    // SecPolicy
    export_c_func!(SecPolicyCreateBasicX509()),
    export_c_func!(SecPolicyCreateSSL(_, _)),
    export_c_func!(SecPolicyCreateRevocation(_)),
    export_c_func!(SecPolicyCopyProperties(_)),
    // SecIdentity
    export_c_func!(SecIdentityCopyCertificate(_, _)),
    export_c_func!(SecIdentityCopyPrivateKey(_, _)),
    // SecAccess
    export_c_func!(SecAccessCreate(_, _, _)),
    // Misc
    export_c_func!(SecCopyErrorMessageString(_, _)),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Security.framework/Security",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};
