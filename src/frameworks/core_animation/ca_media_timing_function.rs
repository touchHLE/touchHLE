/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAMediaTimingFunction`

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::{msg, msg_class};

pub type CAMediaTimingFunctionName = id; // NSString*

pub const kCAMediaTimingFunctionDefault: &str = "default";
pub const kCAMediaTimingFunctionEaseIn: &str = "easeIn";
pub const kCAMediaTimingFunctionEaseInEaseOut: &str = "easeInEaseOut";
pub const kCAMediaTimingFunctionEaseOut: &str = "easeOut";
pub const kCAMediaTimingFunctionLinear: &str = "linear";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCAMediaTimingFunctionDefault",
        HostConstant::NSString(kCAMediaTimingFunctionDefault),
    ),
    (
        "_kCAMediaTimingFunctionEaseIn",
        HostConstant::NSString(kCAMediaTimingFunctionEaseIn),
    ),
    (
        "_kCAMediaTimingFunctionEaseInEaseOut",
        HostConstant::NSString(kCAMediaTimingFunctionEaseInEaseOut),
    ),
    (
        "_kCAMediaTimingFunctionEaseOut",
        HostConstant::NSString(kCAMediaTimingFunctionEaseOut),
    ),
    (
        "_kCAMediaTimingFunctionLinear",
        HostConstant::NSString(kCAMediaTimingFunctionLinear),
    ),
];

#[derive(Default)]
struct CAMediaTimingFunctionHostObject {
    control_points: [[f32; 2]; 2],
}
impl HostObject for CAMediaTimingFunctionHostObject {}
impl CAMediaTimingFunctionHostObject {
    fn solve_for_input(&self, input: f32) -> f32 {
        // My math is kinda rusty so i couldnt solve the equation
        // but a quick google search yielded me people "solving" it
        // by brute forcing it through binary search so... ah well
        let mut lower = 0.0;
        let mut upper = 1.0;
        let mut t;
        let mut x;
        loop {
            t = (upper + lower) / 2.0;
            x = self.coord_in_curve(t, 0);
            if (x - input).abs() < f32::EPSILON {
                break;
            }
            if input > x {
                lower = t;
            } else {
                upper = t;
            }
        }
        return self.coord_in_curve(t, 1);
    }

    fn coord_in_curve(&self, t: f32, x_or_y: usize) -> f32 {
        // 4 control point bezier, knowing the first and last points are (0,0) and (1,1)
        return 3.0 * (1.0 - t).powi(2) * t * self.control_points[0][x_or_y]
            + 3.0 * (1.0 - t) * t.powi(2) * self.control_points[1][x_or_y]
            + t.powi(3);
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CAMediaTimingFunction: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CAMediaTimingFunctionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)functionWithName:(CAMediaTimingFunctionName)name {
    let name_string = to_rust_string(env, name);
    let object: id = match &*name_string {
        kCAMediaTimingFunctionDefault => msg_class![env; CAMediaTimingFunction functionWithControlPoints: 0.25f32 : 0.10f32 : 0.25f32 : 1.00f32],
        kCAMediaTimingFunctionEaseIn => msg_class![env; CAMediaTimingFunction functionWithControlPoints: 0.42f32 : 0.0f32 : 1.0f32 : 1.0f32],
        kCAMediaTimingFunctionEaseInEaseOut => msg_class![env; CAMediaTimingFunction functionWithControlPoints: 0.42f32 : 0.0f32 : 0.58f32 : 1.0f32],
        kCAMediaTimingFunctionEaseOut => msg_class![env; CAMediaTimingFunction functionWithControlPoints: 0.0f32 : 0.0f32 : 0.58f32 : 1.0f32],
        kCAMediaTimingFunctionLinear => msg_class![env; CAMediaTimingFunction functionWithControlPoints: 0.0f32 : 0.0f32 : 1.0f32 : 1.0f32],
        _ => panic!("Attempted to instance CAMediaTimingFunction with unknown name {}", name_string),
    };
    log_dbg!("[CAMediaTimingFunction functionWithName:{:?} ({:?})] -> {:?}", name, name_string, object);
    autorelease(env, object)
}

+ (id)functionWithControlPoints:(f32) c1x
                               :(f32) c1y
                               :(f32) c2x
                               :(f32) c2y {
    let object = msg![env; this alloc];
    msg![env; object initWithControlPoints:c1x : c1y : c2x : c2y]
}

- (id)initWithControlPoints:(f32) c1x
                           :(f32) c1y
                           :(f32) c2x
                           :(f32) c2y {
    let host_object = env.objc.borrow_mut::<CAMediaTimingFunctionHostObject>(this);
    host_object.control_points = [
        [c1x, c1y],
        [c2x, c2y],
    ];
    this
}

// Private method, undocumented.
// Using the original name from the header files in case an app overrides it
- (f32)_solveForInput:(f32) input {
    env.objc.borrow::<CAMediaTimingFunctionHostObject>(this).solve_for_input(input)
}

@end

};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_solve_for_input() {
        let linear = CAMediaTimingFunctionHostObject {
            control_points: [[0.0, 0.0], [1.0, 1.0]],
        };
        {
            let input = 0.0;
            let solution = linear.solve_for_input(input);
            let expected = 0.0;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.25;
            let solution = linear.solve_for_input(input);
            let expected = 0.25;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.50;
            let solution = linear.solve_for_input(input);
            let expected = 0.50;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.75;
            let solution = linear.solve_for_input(input);
            let expected = 0.75;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 1.00;
            let solution = linear.solve_for_input(input);
            let expected = 1.00;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
    }

    #[test]
    fn test_ease_in_ease_out_solve_for_input() {
        let easeInEaseOut = CAMediaTimingFunctionHostObject {
            control_points: [[0.42, 0.0], [0.58, 1.0]],
        };
        {
            let input = 0.00;
            let solution = easeInEaseOut.solve_for_input(input);
            let expected = 0.0000;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.25;
            let solution = easeInEaseOut.solve_for_input(input);
            let expected = 0.1291;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.50;
            let solution = easeInEaseOut.solve_for_input(input);
            let expected = 0.5000;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 0.75;
            let solution = easeInEaseOut.solve_for_input(input);
            let expected = 0.8708;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
        {
            let input = 1.00;
            let solution = easeInEaseOut.solve_for_input(input);
            let expected = 1.000;
            assert!(
                (solution - expected).abs() < 0.0001,
                "For input {} value {}, expected {}",
                input,
                solution,
                expected
            );
        }
    }
}
