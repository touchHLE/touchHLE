use crate::objc::{autorelease, id, msg, nil, release, retain, HostObject};
use crate::objc_classes;

// =====================================================================
// HOST OBJECTS
// =====================================================================

#[derive(Default)]
pub(super) struct CAValueFunctionHostObject {
    name: id,
}

#[derive(Default)]
pub(super) struct CASpringAnimationHostObject {
    // CAAnimation & CAMediaTiming
    duration: f64,
    fill_mode: id,
    delegate: id,
    removed_on_completion: bool,
    timing_function: id,

    // CAPropertyAnimation
    key_path: id,
    is_cumulative: bool,
    is_additive: bool,
    value_function: id,

    // CABasicAnimation
    from_value: id,
    to_value: id,
    by_value: id,

    // CASpringAnimation
    damping: f64,
    initial_velocity: f64,
    mass: f64,
    stiffness: f64,
    allows_overdamping: bool,
    bounce: f64,
    perceptual_duration: f64,
    settling_duration: f64,
}

#[derive(Default)]
pub(super) struct CAKeyframeAnimationHostObject {
    // CAAnimation & CAMediaTiming
    duration: f64,
    fill_mode: id,
    delegate: id,
    removed_on_completion: bool,
    timing_function: id,

    // CAPropertyAnimation
    key_path: id,
    is_cumulative: bool,
    is_additive: bool,
    value_function: id,

    // CAKeyframeAnimation
    values: id,
    path: id,
    key_times: id,
    timing_functions: id,
    calculation_mode: id,
    rotation_mode: id,
    tension_values: id,
    continuity_values: id,
    bias_values: id,
}

impl HostObject for CAValueFunctionHostObject {}
impl HostObject for CASpringAnimationHostObject {}
impl HostObject for CAKeyframeAnimationHostObject {}

// =====================================================================
// CLASSES EXPORT
// =====================================================================

pub const CLASSES: crate::objc::ClassExports = objc_classes! {
    (env, this, _cmd);

    // -----------------------------------------------------------------
    // CAValueFunction
    // -----------------------------------------------------------------
    @implementation CAValueFunction : NSObject
    + (id)alloc {
        let host_object = Box::new(CAValueFunctionHostObject { name: nil });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    + (id)functionWithName:(id)name {
        let func: id = msg![env; this alloc];
        let func: id = msg![env; func init];
        if name != nil {
            () = msg![env; func setName:name];
        }
        autorelease(env, func)
    }

    - (id)init { this }

    - (())dealloc {
        let name = env.objc.borrow::<CAValueFunctionHostObject>(this).name;
        if name != nil { release(env, name); }
        env.objc.dealloc_object(this, &mut env.mem)
    }

    - (id)name { env.objc.borrow::<CAValueFunctionHostObject>(this).name }
    - (())setName:(id)val {
        let old = env.objc.borrow::<CAValueFunctionHostObject>(this).name;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAValueFunctionHostObject>(this).name = val;
        if old != nil { release(env, old); }
    }
    @end

    // -----------------------------------------------------------------
    // CASpringAnimation
    // -----------------------------------------------------------------
    @implementation CASpringAnimation : NSObject
    + (id)alloc {
        let host_object = Box::new(CASpringAnimationHostObject {
            duration: 0.0, fill_mode: nil, delegate: nil, removed_on_completion: true, timing_function: nil,
            key_path: nil, is_cumulative: false, is_additive: false, value_function: nil,
            from_value: nil, to_value: nil, by_value: nil,
            damping: 10.0, initial_velocity: 0.0, mass: 1.0, stiffness: 100.0,
            allows_overdamping: false, bounce: 0.0, perceptual_duration: 0.0, settling_duration: 0.0,
        });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    + (id)animationWithKeyPath:(id)path {
        let anim: id = msg![env; this alloc];
        let anim: id = msg![env; anim init];
        if path != nil { () = msg![env; anim setKeyPath:path]; }
        autorelease(env, anim)
    }

    - (id)init { this }

    - (())dealloc {
        let (
            fill_mode, delegate, timing_function, key_path, value_function,
            from_value, to_value, by_value
        ) = {
            let host = env.objc.borrow::<CASpringAnimationHostObject>(this);
            (
                host.fill_mode, host.delegate, host.timing_function, host.key_path, host.value_function,
                host.from_value, host.to_value, host.by_value
            )
        };

        if fill_mode != nil { release(env, fill_mode); }
        if delegate != nil { release(env, delegate); }
        if timing_function != nil { release(env, timing_function); }
        if key_path != nil { release(env, key_path); }
        if value_function != nil { release(env, value_function); }
        if from_value != nil { release(env, from_value); }
        if to_value != nil { release(env, to_value); }
        if by_value != nil { release(env, by_value); }

        env.objc.dealloc_object(this, &mut env.mem)
    }

    // CAAnimation + CAPropertyAnimation + CABasicAnimation properties
    // (Повторяем базовые геттеры/сеттеры для CASpringAnimation)
    - (f64)duration { env.objc.borrow::<CASpringAnimationHostObject>(this).duration }
    - (())setDuration:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).duration = val; }

    - (bool)isRemovedOnCompletion { env.objc.borrow::<CASpringAnimationHostObject>(this).removed_on_completion }
    - (())setRemovedOnCompletion:(bool)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).removed_on_completion = val; }

    - (id)fillMode { env.objc.borrow::<CASpringAnimationHostObject>(this).fill_mode }
    - (())setFillMode:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).fill_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).fill_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)delegate { env.objc.borrow::<CASpringAnimationHostObject>(this).delegate }
    - (())setDelegate:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).delegate;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).delegate = val;
        if old != nil { release(env, old); }
    }

    - (id)timingFunction { env.objc.borrow::<CASpringAnimationHostObject>(this).timing_function }
    - (())setTimingFunction:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).timing_function;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).timing_function = val;
        if old != nil { release(env, old); }
    }

    - (id)keyPath { env.objc.borrow::<CASpringAnimationHostObject>(this).key_path }
    - (())setKeyPath:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).key_path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).key_path = val;
        if old != nil { release(env, old); }
    }

    - (bool)isCumulative { env.objc.borrow::<CASpringAnimationHostObject>(this).is_cumulative }
    - (())setCumulative:(bool)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).is_cumulative = val; }

    - (bool)isAdditive { env.objc.borrow::<CASpringAnimationHostObject>(this).is_additive }
    - (())setAdditive:(bool)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).is_additive = val; }

    - (id)valueFunction { env.objc.borrow::<CASpringAnimationHostObject>(this).value_function }
    - (())setValueFunction:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).value_function;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).value_function = val;
        if old != nil { release(env, old); }
    }

    - (id)fromValue { env.objc.borrow::<CASpringAnimationHostObject>(this).from_value }
    - (())setFromValue:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).from_value;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).from_value = val;
        if old != nil { release(env, old); }
    }

    - (id)toValue { env.objc.borrow::<CASpringAnimationHostObject>(this).to_value }
    - (())setToValue:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).to_value;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).to_value = val;
        if old != nil { release(env, old); }
    }

    - (id)byValue { env.objc.borrow::<CASpringAnimationHostObject>(this).by_value }
    - (())setByValue:(id)val {
        let old = env.objc.borrow::<CASpringAnimationHostObject>(this).by_value;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CASpringAnimationHostObject>(this).by_value = val;
        if old != nil { release(env, old); }
    }

    // CASpringAnimation properties
    - (f64)damping { env.objc.borrow::<CASpringAnimationHostObject>(this).damping }
    - (())setDamping:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).damping = val; }

    - (f64)initialVelocity { env.objc.borrow::<CASpringAnimationHostObject>(this).initial_velocity }
    - (())setInitialVelocity:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).initial_velocity = val; }

    - (f64)mass { env.objc.borrow::<CASpringAnimationHostObject>(this).mass }
    - (())setMass:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).mass = val; }

    - (f64)stiffness { env.objc.borrow::<CASpringAnimationHostObject>(this).stiffness }
    - (())setStiffness:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).stiffness = val; }

    - (bool)allowsOverdamping { env.objc.borrow::<CASpringAnimationHostObject>(this).allows_overdamping }
    - (())setAllowsOverdamping:(bool)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).allows_overdamping = val; }

    - (f64)bounce { env.objc.borrow::<CASpringAnimationHostObject>(this).bounce }
    - (())setBounce:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).bounce = val; }

    - (f64)perceptualDuration { env.objc.borrow::<CASpringAnimationHostObject>(this).perceptual_duration }
    - (())setPerceptualDuration:(f64)val { env.objc.borrow_mut::<CASpringAnimationHostObject>(this).perceptual_duration = val; }

    - (f64)settlingDuration { env.objc.borrow::<CASpringAnimationHostObject>(this).settling_duration }
    @end

    // -----------------------------------------------------------------
    // CAKeyframeAnimation (Updated)
    // -----------------------------------------------------------------
    @implementation CAKeyframeAnimation : NSObject
    + (id)alloc {
        let host_object = Box::new(CAKeyframeAnimationHostObject {
            duration: 0.0, fill_mode: nil, delegate: nil, removed_on_completion: true, timing_function: nil,
            key_path: nil, is_cumulative: false, is_additive: false, value_function: nil,
            values: nil, path: nil, key_times: nil, timing_functions: nil, calculation_mode: nil,
            rotation_mode: nil, tension_values: nil, continuity_values: nil, bias_values: nil,
        });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    + (id)animationWithKeyPath:(id)path {
        let anim: id = msg![env; this alloc];
        let anim: id = msg![env; anim init];
        if path != nil { () = msg![env; anim setKeyPath:path]; }
        autorelease(env, anim)
    }

    - (id)init { this }

    - (())dealloc {
        let (
            fill_mode, delegate, timing_function, key_path, value_function,
            values, path, key_times, timing_functions, calculation_mode,
            rotation_mode, tension_values, continuity_values, bias_values
        ) = {
            let host = env.objc.borrow::<CAKeyframeAnimationHostObject>(this);
            (
                host.fill_mode, host.delegate, host.timing_function, host.key_path, host.value_function,
                host.values, host.path, host.key_times, host.timing_functions, host.calculation_mode,
                host.rotation_mode, host.tension_values, host.continuity_values, host.bias_values
            )
        };

        if fill_mode != nil { release(env, fill_mode); }
        if delegate != nil { release(env, delegate); }
        if timing_function != nil { release(env, timing_function); }
        if key_path != nil { release(env, key_path); }
        if value_function != nil { release(env, value_function); }
        if values != nil { release(env, values); }
        if path != nil { release(env, path); }
        if key_times != nil { release(env, key_times); }
        if timing_functions != nil { release(env, timing_functions); }
        if calculation_mode != nil { release(env, calculation_mode); }
        if rotation_mode != nil { release(env, rotation_mode); }
        if tension_values != nil { release(env, tension_values); }
        if continuity_values != nil { release(env, continuity_values); }
        if bias_values != nil { release(env, bias_values); }

        env.objc.dealloc_object(this, &mut env.mem)
    }

    // CAAnimation + CAPropertyAnimation
    - (f64)duration { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).duration }
    - (())setDuration:(f64)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = val; }

    - (bool)isRemovedOnCompletion { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).removed_on_completion }
    - (())setRemovedOnCompletion:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).removed_on_completion = val; }

    - (id)fillMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).fill_mode }
    - (())setFillMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).fill_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).fill_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)delegate { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate }
    - (())setDelegate:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).delegate = val;
        if old != nil { release(env, old); }
    }

    - (id)timingFunction { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_function }
    - (())setTimingFunction:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_function;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).timing_function = val;
        if old != nil { release(env, old); }
    }

    - (id)keyPath { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path }
    - (())setKeyPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_path = val;
        if old != nil { release(env, old); }
    }

    - (bool)isCumulative { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).is_cumulative }
    - (())setCumulative:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).is_cumulative = val; }

    - (bool)isAdditive { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).is_additive }
    - (())setAdditive:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).is_additive = val; }

    - (id)valueFunction { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).value_function }
    - (())setValueFunction:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).value_function;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).value_function = val;
        if old != nil { release(env, old); }
    }

    // CAKeyframeAnimation properties
    - (id)values { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values }
    - (())setValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values = val;
        if old != nil { release(env, old); }
    }

    - (id)path { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).path }
    - (())setPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).path = val;
        if old != nil { release(env, old); }
    }

    - (id)keyTimes { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times }
    - (())setKeyTimes:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_times = val;
        if old != nil { release(env, old); }
    }

    - (id)timingFunctions { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_functions }
    - (())setTimingFunctions:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_functions;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).timing_functions = val;
        if old != nil { release(env, old); }
    }

    - (id)calculationMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).calculation_mode }
    - (())setCalculationMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).calculation_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).calculation_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)rotationMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).rotation_mode }
    - (())setRotationMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).rotation_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).rotation_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)tensionValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).tension_values }
    - (())setTensionValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).tension_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).tension_values = val;
        if old != nil { release(env, old); }
    }

    - (id)continuityValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).continuity_values }
    - (())setContinuityValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).continuity_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).continuity_values = val;
        if old != nil { release(env, old); }
    }

    - (id)biasValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).bias_values }
    - (())setBiasValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).bias_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).bias_values = val;
        if old != nil { release(env, old); }
    }
    @end

};
