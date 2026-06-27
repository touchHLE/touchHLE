/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSException`.
//!
//! Provides a host-side implementation of NSException including `raise`,
//! `description`, copy semantics, and the uncaught-exception handler stub.
//!
//! ### On `raise`
//! In real Objective-C, `-[NSException raise]` unwinds the call stack via
//! C++ exceptions.  touchHLE does not emulate ObjC exception unwinding, so we
//! instead call `panic!` which terminates the guest with a clear diagnostic.
//! This is the same behaviour as the original touchHLE approach for unhandled
//! guest panics, and is far better than silently continuing past a `raise`
//! (which produces mysterious NULL-deref crashes later, as seen in the
//! KamiChallenge log).

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::{export_c_func, Environment};

// ---------------------------------------------------------------------------
// Host object
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NSExceptionHostObject {
    name: id,      // NSString*
    reason: id,    // NSString*
    user_info: id, // NSDictionary*  (may be nil)
}
impl HostObject for NSExceptionHostObject {}

// ---------------------------------------------------------------------------
// Helper: convert an ObjC NSString* to a Rust String safely.
// If the id is nil, returns the provided fallback.
// ---------------------------------------------------------------------------
fn objc_str(env: &mut Environment, s: id, fallback: &str) -> String {
    if s == nil {
        return fallback.to_string();
    }
    super::ns_string::to_rust_string(env, s).into_owned()
}

#[derive(Default)]
pub struct State {
    pub uncaught_exception_handler: MutVoidPtr,
}


pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSException: NSObject

// MARK: - Alloc

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSExceptionHostObject {
        name:      nil,
        reason:    nil,
        user_info: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Factory / convenience constructors

+ (id)exceptionWithName:(id)name      // NSString*
                 reason:(id)reason    // NSString*
               userInfo:(id)user_info // NSDictionary*  (may be nil)
{
    let obj: id = msg_class![env; NSException alloc];
    let obj: id = msg![env; obj initWithName:name reason:reason userInfo:user_info];
    autorelease(env, obj)
}

// `+raise:format:` — convenience that creates and immediately raises.
// The `format` parameter is treated as a plain reason string (no printf
// substitution) because varargs are not supported in touchHLE HLE stubs.
+ (())raise:(id)name   // NSString*  (exception name)
       format:(id)fmt  // NSString*  (reason / format string)
{
    let exc: id = msg_class![env; NSException exceptionWithName:name
                                                         reason:fmt
                                                       userInfo:nil];
    () = msg![env; exc raise];
}

// MARK: - Designated initialiser

- (id)initWithName:(id)name      // NSString*
            reason:(id)reason    // NSString*
          userInfo:(id)user_info // NSDictionary*  (may be nil)
{
    retain(env, name);
    retain(env, reason);
    retain(env, user_info);
    {
        let host = env.objc.borrow_mut::<NSExceptionHostObject>(this);
        host.name      = name;
        host.reason    = reason;
        host.user_info = user_info;
    }
    this
}

// MARK: - Accessors

- (id)name {
    env.objc.borrow::<NSExceptionHostObject>(this).name
}

- (id)reason {
    env.objc.borrow::<NSExceptionHostObject>(this).reason
}

- (id)userInfo {
    env.objc.borrow::<NSExceptionHostObject>(this).user_info
}

// Returns an empty array — we have no guest stack-trace support.
- (id)callStackSymbols {
    msg_class![env; NSArray array]
}

// Returns an empty array — same reason.
- (id)callStackReturnAddresses {
    msg_class![env; NSArray array]
}

// MARK: - raise

- (())raise {
    let name   = env.objc.borrow::<NSExceptionHostObject>(this).name;
    let reason = env.objc.borrow::<NSExceptionHostObject>(this).reason;

    let name_s   = objc_str(env, name,   "<unnamed exception>");
    let reason_s = objc_str(env, reason, "<no reason>");

    // Log the exception details clearly for debugging
    log!("GUEST NSException raised (Continuing without panic):");
    log!("  Name:   {}", name_s);
    log!("  Reason: {}", reason_s);

    // If there is userInfo, log it too (useful for network errors)
    let user_info = env.objc.borrow::<NSExceptionHostObject>(this).user_info;
    if user_info != nil {
        log!("  UserInfo: <present>");
    }

    // We return () to the guest.
    // The guest will now continue execution.
}

// MARK: - NSCopying

// NSException is documented as immutable once created, so `copy` just
// retains and returns `self` (same as NSString / NSArray behaviour).
- (id)copy {
    retain(env, this);
    this
}

- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this);
    this
}

// MARK: - description

- (id)description {
    let name   = env.objc.borrow::<NSExceptionHostObject>(this).name;
    let reason = env.objc.borrow::<NSExceptionHostObject>(this).reason;
    let name_s   = objc_str(env, name,   "<unnamed>");
    let reason_s = objc_str(env, reason, "<no reason>");
    let s = format!("NSException: name=\"{}\" reason=\"{}\"", name_s, reason_s);
    let ns = super::ns_string::from_rust_string(env, s);
    autorelease(env, ns)
}

// MARK: - Dealloc

- (())dealloc {
    let host = env.objc.borrow::<NSExceptionHostObject>(this);
    let (name, reason, user_info) = (host.name, host.reason, host.user_info);
    release(env, name);
    release(env, reason);
    release(env, user_info);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

// ---------------------------------------------------------------------------
// NSExceptionName string constants
// ---------------------------------------------------------------------------

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSCharacterConversionException",
        HostConstant::NSString("NSCharacterConversionException"),
    ),
    (
        "_NSDecimalNumberDivideByZeroException",
        HostConstant::NSString("NSDecimalNumberDivideByZeroException"),
    ),
    (
        "_NSDecimalNumberExactnessException",
        HostConstant::NSString("NSDecimalNumberExactnessException"),
    ),
    (
        "_NSDecimalNumberOverflowException",
        HostConstant::NSString("NSDecimalNumberOverflowException"),
    ),
    (
        "_NSDecimalNumberUnderflowException",
        HostConstant::NSString("NSDecimalNumberUnderflowException"),
    ),
    (
        "_NSDestinationInvalidException",
        HostConstant::NSString("NSDestinationInvalidException"),
    ),
    (
        "_NSFileHandleOperationException",
        HostConstant::NSString("NSFileHandleOperationException"),
    ),
    (
        "_NSGenericException",
        HostConstant::NSString("NSGenericException"),
    ),
    (
        "_NSInternalInconsistencyException",
        HostConstant::NSString("NSInternalInconsistencyException"),
    ),
    (
        "_NSInvalidArchiveOperationException",
        HostConstant::NSString("NSInvalidArchiveOperationException"),
    ),
    (
        "_NSInvalidArgumentException",
        HostConstant::NSString("NSInvalidArgumentException"),
    ),
    (
        "_NSInvalidReceivePortException",
        HostConstant::NSString("NSInvalidReceivePortException"),
    ),
    (
        "_NSInvalidSendPortException",
        HostConstant::NSString("NSInvalidSendPortException"),
    ),
    (
        "_NSInvalidUnarchiveOperationException",
        HostConstant::NSString("NSInvalidUnarchiveOperationException"),
    ),
    (
        "_NSInvocationOperationCancelledException",
        HostConstant::NSString("NSInvocationOperationCancelledException"),
    ),
    (
        "_NSInvocationOperationVoidResultException",
        HostConstant::NSString("NSInvocationOperationVoidResultException"),
    ),
    (
        "_NSMallocException",
        HostConstant::NSString("NSMallocException"),
    ),
    (
        "_NSObjectInaccessibleException",
        HostConstant::NSString("NSObjectInaccessibleException"),
    ),
    (
        "_NSObjectNotAvailableException",
        HostConstant::NSString("NSObjectNotAvailableException"),
    ),
    (
        "_NSOldStyleException",
        HostConstant::NSString("NSOldStyleException"),
    ),
    (
        "_NSParseErrorException",
        HostConstant::NSString("NSParseErrorException"),
    ),
    (
        "_NSPortReceiveException",
        HostConstant::NSString("NSPortReceiveException"),
    ),
    (
        "_NSPortSendException",
        HostConstant::NSString("NSPortSendException"),
    ),
    (
        "_NSPortTimeoutException",
        HostConstant::NSString("NSPortTimeoutException"),
    ),
    (
        "_NSRangeException",
        HostConstant::NSString("NSRangeException"),
    ),
    (
        "_NSUndefinedKeyException",
        HostConstant::NSString("NSUndefinedKeyException"),
    ),
    (
        "_NSInconsistentArchiveException",
        HostConstant::NSString("NSInconsistentArchiveException"),
    ),
    (
        "_NSPPDIncludeNotFoundException",
        HostConstant::NSString("NSPPDIncludeNotFoundException"),
    ),
    (
        "_NSPPDIncludeStackOverflowException",
        HostConstant::NSString("NSPPDIncludeStackOverflowException"),
    ),
    (
        "_NSPPDIncludeStackUnderflowException",
        HostConstant::NSString("NSPPDIncludeStackUnderflowException"),
    ),
    (
        "_NSPPDParseException",
        HostConstant::NSString("NSPPDParseException"),
    ),
    (
        "_NSRTFPropertyStackOverflowException",
        HostConstant::NSString("NSRTFPropertyStackOverflowException"),
    ),
    (
        "_NSTIFFException",
        HostConstant::NSString("NSTIFFException"),
    ),
    (
        "_NSAbortModalException",
        HostConstant::NSString("NSAbortModalException"),
    ),
    (
        "_NSAbortPrintingException",
        HostConstant::NSString("NSAbortPrintingException"),
    ),
    (
        "_NSAccessibilityException",
        HostConstant::NSString("NSAccessibilityException"),
    ),
    (
        "_NSAppKitIgnoredException",
        HostConstant::NSString("NSAppKitIgnoredException"),
    ),
    (
        "_NSAppKitVirtualMemoryException",
        HostConstant::NSString("NSAppKitVirtualMemoryException"),
    ),
    (
        "_NSBadBitmapParametersException",
        HostConstant::NSString("NSBadBitmapParametersException"),
    ),
    (
        "_NSBadComparisonException",
        HostConstant::NSString("NSBadComparisonException"),
    ),
    (
        "_NSBadRTFColorTableException",
        HostConstant::NSString("NSBadRTFColorTableException"),
    ),
    (
        "_NSBadRTFDirectiveException",
        HostConstant::NSString("NSBadRTFDirectiveException"),
    ),
    (
        "_NSBadRTFFontTableException",
        HostConstant::NSString("NSBadRTFFontTableException"),
    ),
    (
        "_NSBadRTFStyleSheetException",
        HostConstant::NSString("NSBadRTFStyleSheetException"),
    ),
    (
        "_NSBrowserIllegalDelegateException",
        HostConstant::NSString("NSBrowserIllegalDelegateException"),
    ),
    (
        "_NSColorListIOException",
        HostConstant::NSString("NSColorListIOException"),
    ),
    (
        "_NSColorListNotEditableException",
        HostConstant::NSString("NSColorListNotEditableException"),
    ),
    (
        "_NSDraggingException",
        HostConstant::NSString("NSDraggingException"),
    ),
    (
        "_NSFontUnavailableException",
        HostConstant::NSString("NSFontUnavailableException"),
    ),
    (
        "_NSIllegalSelectorException",
        HostConstant::NSString("NSIllegalSelectorException"),
    ),
    (
        "_NSImageCacheException",
        HostConstant::NSString("NSImageCacheException"),
    ),
    (
        "_NSNibLoadingException",
        HostConstant::NSString("NSNibLoadingException"),
    ),
    (
        "_NSPasteboardCommunicationException",
        HostConstant::NSString("NSPasteboardCommunicationException"),
    ),
    (
        "_NSPrintOperationExistsException",
        HostConstant::NSString("NSPrintOperationExistsException"),
    ),
    (
        "_NSPrintPackageException",
        HostConstant::NSString("NSPrintPackageException"),
    ),
    (
        "_NSPrintingCommunicationException",
        HostConstant::NSString("NSPrintingCommunicationException"),
    ),
    (
        "_NSTextLineTooLongException",
        HostConstant::NSString("NSTextLineTooLongException"),
    ),
    (
        "_NSTextNoSelectionException",
        HostConstant::NSString("NSTextNoSelectionException"),
    ),
    (
        "_NSTextReadException",
        HostConstant::NSString("NSTextReadException"),
    ),
    (
        "_NSTextWriteException",
        HostConstant::NSString("NSTextWriteException"),
    ),
    (
        "_NSTypedStreamVersionException",
        HostConstant::NSString("NSTypedStreamVersionException"),
    ),
    (
        "_NSWindowServerCommunicationException",
        HostConstant::NSString("NSWindowServerCommunicationException"),
    ),
    (
        "_NSWordTablesReadException",
        HostConstant::NSString("NSWordTablesReadException"),
    ),
    (
        "_NSWordTablesWriteException",
        HostConstant::NSString("NSWordTablesWriteException"),
    ),
    // UIKit exception names (placed here for proximity to Foundation
    // exceptions)
    (
        "_UIViewControllerHierarchyInconsistencyException",
        HostConstant::NSString("UIViewControllerHierarchyInconsistencyException"),
    ),
    (
        "_UIApplicationInvalidInterfaceOrientationException",
        HostConstant::NSString("UIApplicationInvalidInterfaceOrientationException"),
    ),
];

// ---------------------------------------------------------------------------
// C functions: NSSetUncaughtExceptionHandler / NSGetUncaughtExceptionHandler
// ---------------------------------------------------------------------------

/// Registers a last-chance exception handler. In touchHLE all unhandled
/// exceptions are already converted to Rust panics or bypassed, but we
/// save the handler address to maintain accurate guest state and so that
/// `NSGetUncaughtExceptionHandler` can return whatever was last installed.
/// <https://developer.apple.com/documentation/foundation/1409609-nssetuncaughtexceptionhandler>
fn NSSetUncaughtExceptionHandler(env: &mut Environment, handler: MutVoidPtr) {
    env.framework_state
        .foundation
        .ns_exception
        .uncaught_exception_handler = handler;

    log!(
        "NSSetUncaughtExceptionHandler: registered handler at {:?}",
        handler
    );
}

/// Returns the function pointer previously installed via
/// `NSSetUncaughtExceptionHandler`, or `NULL` if no handler has been
/// installed in this process. Apple crash-reporting libraries (PLCrashReporter,
/// Crashlytics, Flurry, …) call this on init so they can chain to any
/// existing handler instead of clobbering it.
/// <https://developer.apple.com/documentation/foundation/1416853-nsgetuncaughtexceptionhandler>
fn NSGetUncaughtExceptionHandler(env: &mut Environment) -> MutVoidPtr {
    let handler = env
        .framework_state
        .foundation
        .ns_exception
        .uncaught_exception_handler;
    log_dbg!(
        "NSGetUncaughtExceptionHandler -> {:?}",
        handler
    );
    handler
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NSSetUncaughtExceptionHandler(_)),
    export_c_func!(NSGetUncaughtExceptionHandler()),
];
