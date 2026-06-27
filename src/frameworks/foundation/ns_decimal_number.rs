/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDecimalNumber` and `NSDecimal`.

use super::ns_string::{from_rust_string, to_rust_string};
use super::NSUInteger;
use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

// MARK: - Rounding modes

type NSRoundingMode = NSUInteger;
const NSRoundPlain: NSRoundingMode = 0; // round half up
const NSRoundDown: NSRoundingMode = 1;
const NSRoundUp: NSRoundingMode = 2;
const NSRoundBankers: NSRoundingMode = 3; // round half to even

// MARK: - Calculation error codes

type NSCalculationError = NSUInteger;
const NSCalculationNoError: NSCalculationError = 0;
const NSCalculationLossOfPrecision: NSCalculationError = 1;
const NSCalculationUnderflow: NSCalculationError = 2;
const NSCalculationOverflow: NSCalculationError = 3;
const NSCalculationDivideByZero: NSCalculationError = 4;

// MARK: - Host object

/// We store the value as an `f64` for simplicity.  Full BCD decimal storage
/// is not needed for the game use-cases touchHLE targets.
#[derive(Default)]
struct NSDecimalNumberHostObject {
    value: f64,
}
impl HostObject for NSDecimalNumberHostObject {}

// MARK: - NSDecimalNumberHandler host object

#[derive(Default)]
struct NSDecimalNumberHandlerHostObject {
    rounding_mode: NSRoundingMode,
    scale: i16,
    raise_on_exactness: bool,
    raise_on_overflow: bool,
    raise_on_underflow: bool,
    raise_on_divide_by_zero: bool,
}
impl HostObject for NSDecimalNumberHandlerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// NSDecimalNumber
// =========================================================================

@implementation NSDecimalNumber: NSNumber

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDecimalNumberHostObject { value: 0.0 });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Sentinel values

+ (id)zero {
    let n: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(n).value = 0.0;
    autorelease(env, n)
}

+ (id)one {
    let n: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(n).value = 1.0;
    autorelease(env, n)
}

+ (id)minimumDecimalNumber {
    let n: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(n).value = f64::MIN;
    autorelease(env, n)
}

+ (id)maximumDecimalNumber {
    let n: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(n).value = f64::MAX;
    autorelease(env, n)
}

// MARK: - Default behavior

+ (id)defaultBehavior {
    msg_class![env; NSDecimalNumberHandler defaultDecimalNumberHandler]
}

+ (id)notANumber {
    let n: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(n).value = f64::NAN;
    autorelease(env, n)
}

// MARK: - Constructors

+ (id)decimalNumberWithString:(id)string { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithString:string];
    autorelease(env, new)
}

+ (id)decimalNumberWithString:(id)string
                       locale:(id)_locale { // NSString*, id
    msg_class![env; NSDecimalNumber decimalNumberWithString:string]
}

+ (id)decimalNumberWithDecimal:(NSDecimal)decimal {
    // NSDecimal is passed by pointer in the C API but as a value in ObjC.
    // We receive it as a raw bytes blob via the host; treat as f64.
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(new).value = decimal as f64;
    autorelease(env, new)
}

+ (id)decimalNumberWithMantissa:(u64)mantissa
                       exponent:(i16)exponent
                     isNegative:(bool)is_negative {
    let new: id = msg![env; this alloc];
    let value = mantissa as f64 * 10f64.powi(exponent as i32)
        * if is_negative { -1.0 } else { 1.0 };
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(new).value = value;
    autorelease(env, new)
}

- (id)initWithString:(id)string { // NSString*
    if string == nil {
        release(env, this);
        return nil;
    }
    let s = to_rust_string(env, string).into_owned();
    let value = s.trim().parse::<f64>().unwrap_or(0.0);
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(this).value = value;
    this
}

- (id)initWithString:(id)string locale:(id)_locale {
    msg![env; this initWithString:string]
}

- (id)initWithDecimal:(NSDecimal)decimal {
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(this).value = decimal as f64;
    this
}

- (id)initWithMantissa:(u64)mantissa
              exponent:(i16)exponent
            isNegative:(bool)is_negative {
    let value = mantissa as f64 * 10f64.powi(exponent as i32)
        * if is_negative { -1.0 } else { 1.0 };
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(this).value = value;
    this
}

// Convenience init from double.
- (id)initWithDouble:(f64)value {
    env.objc.borrow_mut::<NSDecimalNumberHostObject>(this).value = value;
    this
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - NSNumber primitive overrides

- (bool)boolValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value != 0.0
}

- (i32)intValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as i32
}

- (NSInteger)integerValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as NSInteger
}

- (i32)longValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as i32
}

- (i64)longLongValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as i64
}

- (u32)unsignedIntValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as u32
}

- (NSUInteger)unsignedIntegerValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as NSUInteger
}

- (u64)unsignedLongLongValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as u64
}

- (f32)floatValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as f32
}

- (f64)doubleValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value
}

- (i16)shortValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as i16
}

- (u16)unsignedShortValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as u16
}

- (i8)charValue {
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as i8
}

// MARK: - Arithmetic

- (id)decimalNumberByAdding:(id)other { // NSDecimalNumber*
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    let result = decimal_from_f64(env, a + b);
    autorelease(env, result)
}

- (id)decimalNumberBySubtracting:(id)other {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    let result = decimal_from_f64(env, a - b);
    autorelease(env, result)
}

- (id)decimalNumberByMultiplyingBy:(id)other {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    let result = decimal_from_f64(env, a * b);
    autorelease(env, result)
}

- (id)decimalNumberByDividingBy:(id)other {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    if b == 0.0 {
        log!("NSDecimalNumber: divide by zero");
        return msg_class![env; NSDecimalNumber notANumber];
    }
    let result = decimal_from_f64(env, a / b);
    autorelease(env, result)
}

- (id)decimalNumberByRaisingToPower:(NSUInteger)power {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let result = decimal_from_f64(env, a.powi(power as i32));
    autorelease(env, result)
}

- (id)decimalNumberByMultiplyingByPowerOf10:(i16)power {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let result = decimal_from_f64(env, a * 10f64.powi(power as i32));
    autorelease(env, result)
}

// Arithmetic with behaviour (behaviour controls rounding/error — ignored here).
- (id)decimalNumberByAdding:(id)other withBehavior:(id)_behavior {
    msg![env; this decimalNumberByAdding:other]
}
- (id)decimalNumberBySubtracting:(id)other withBehavior:(id)_behavior {
    msg![env; this decimalNumberBySubtracting:other]
}
- (id)decimalNumberByMultiplyingBy:(id)other withBehavior:(id)_behavior {
    msg![env; this decimalNumberByMultiplyingBy:other]
}
- (id)decimalNumberByDividingBy:(id)other withBehavior:(id)_behavior {
    msg![env; this decimalNumberByDividingBy:other]
}
- (id)decimalNumberByRaisingToPower:(NSUInteger)power withBehavior:(id)_behavior {
    msg![env; this decimalNumberByRaisingToPower:power]
}
- (id)decimalNumberByMultiplyingByPowerOf10:(i16)power withBehavior:(id)_behavior {
    msg![env; this decimalNumberByMultiplyingByPowerOf10:power]
}

// MARK: - Rounding

- (id)decimalNumberByRoundingAccordingToBehavior:(id)_behavior {
    // Round to nearest integer as a safe default.
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let result = decimal_from_f64(env, a.round());
    autorelease(env, result)
}

// MARK: - Comparison

- (NSComparisonResult)compare:(id)other {
    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    if a < b  { return -1; } // NSOrderedAscending
    if a > b  { return  1; } // NSOrderedDescending
    0                        // NSOrderedSame
}

- (bool)isEqual:(id)other {
    if this == other { return true; }
    if other == nil  { return false; }

    // ИСПРАВЛЕНИЕ: Безопасная проверка класса вместо try_borrow
    let other_class: crate::objc::Class = msg![env; other class];
    let ns_decimal_class = env.objc.get_known_class("NSDecimalNumber", &mut env.mem);
    if !env.objc.class_is_subclass_of(other_class, ns_decimal_class) {
        return false;
    }

    let a = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let b = env.objc.borrow::<NSDecimalNumberHostObject>(other).value;
    a == b
}

// MARK: - Accessors

- (NSDecimal)decimalValue {
    // NSDecimal is a complex C struct; return value bits as f64.
    env.objc.borrow::<NSDecimalNumberHostObject>(this).value as NSDecimal
}

- (id)objCType {
    // Type encoding for double.
    ns_string::get_static_str(env, "d")
}

// MARK: - String representation

- (id)stringValue {
    msg![env; this description]
}

- (id)descriptionWithLocale:(id)_locale {
    msg![env; this description]
}

- (id)description {
    let value = env.objc.borrow::<NSDecimalNumberHostObject>(this).value;
    let s = if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    let ns = from_rust_string(env, s);
    autorelease(env, ns)
}

// MARK: - NSCopying

- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

@end

// =========================================================================
// NSDecimalNumberHandler
// =========================================================================

@implementation NSDecimalNumberHandler: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDecimalNumberHandlerHostObject {
        rounding_mode: NSRoundPlain,
        scale: 38,
        raise_on_exactness: false,
        raise_on_overflow: true,
        raise_on_underflow: true,
        raise_on_divide_by_zero: true,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)defaultDecimalNumberHandler {
    env.objc.alloc_static_object(
        this,
        Box::new(NSDecimalNumberHandlerHostObject {
            rounding_mode: NSRoundPlain,
            scale: 38,
            raise_on_exactness: false,
            raise_on_overflow: true,
            raise_on_underflow: true,
            raise_on_divide_by_zero: true,
        }),
        &mut env.mem,
    )
}

// ИСПРАВЛЕНИЕ: Удалены методы инициализации с 6 аргументами,
// так как они превышают лимит макросов ядра и всё равно не используются.

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

- (NSRoundingMode)roundingMode {
    env.objc.borrow::<NSDecimalNumberHandlerHostObject>(this).rounding_mode
}

- (i16)scale {
    env.objc.borrow::<NSDecimalNumberHandlerHostObject>(this).scale
}

// NSDecimalNumberBehaviors protocol stubs.
// (roundingMode is defined above; not duplicated here.)

- (id)exceptionDuringOperation:(id)_operation
                          error:(NSCalculationError)error
                    leftOperand:(id)_left
                   rightOperand:(id)_right {
    log!(
        "NSDecimalNumberHandler exceptionDuringOperation: error={} — returning notANumber",
        error
    );
    msg_class![env; NSDecimalNumber notANumber]
}

@end

};

// MARK: - Type alias (NSDecimal is an opaque struct — we use f64 as a proxy)
type NSDecimal = f64;
type NSComparisonResult = NSInteger;

// MARK: - Helpers

fn decimal_from_f64(env: &mut crate::Environment, value: f64) -> crate::objc::id {
    let class = env.objc.get_known_class("NSDecimalNumber", &mut env.mem);
    
    env.objc.alloc_object(
        class,
        Box::new(NSDecimalNumberHostObject { value }),
        &mut env.mem,
    )
}
