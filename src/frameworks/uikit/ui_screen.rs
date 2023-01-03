//! `UIScreen`.

use crate::objc::{id, objc_classes, ClassExports, TrivialHostObject};

#[derive(Default)]
pub struct State {
    main_screen: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIScreen: NSObject

// For now this is a singleton (the only instance is returned by mainScreen)
+ (id)mainScreen {
    if let Some(screen) = env.framework_state.uikit.ui_screen.main_screen {
        screen
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem
        );
        env.framework_state.uikit.ui_screen.main_screen = Some(new);
        new
   }
}
- (id)retain { this }
- (())release {}
- (id)autorelease { this }

// TODO: accessors

@end

};
