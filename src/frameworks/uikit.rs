//! The UIKit framework.
//!
//! For the time being the focus of this project is on running games, which are
//! likely to use UIKit in very simple and limited ways, so this implementation
//! will probably take a lot of shortcuts.

use crate::dyld::FunctionExports;
use crate::Environment;

pub mod ui_application;
pub mod ui_responder;

pub const FUNCTIONS: FunctionExports = &[(
    "_UIApplicationMain",
    &(ui_application::UIApplicationMain as fn(&mut Environment, _, _, _, _)),
)];
