/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Separate module just for the dylib list, so it gets its own git history.

use crate::frameworks;
use crate::libc;
use crate::objc;

/// The single list of host dylibs that the linker (and Objective-C runtime)
/// searches through.
pub const DYLIB_LIST: &[&super::HostDylib] = &[
    &libc::DYLIB,
    &objc::DYLIB,
    &crate::environment::app_picker::DYLIB, // Not a real library; special internal classes.
    &frameworks::audio_toolbox::DYLIB,
    &frameworks::avfoundation::DYLIB,
    &frameworks::core_animation::DYLIB,
    &frameworks::core_foundation::DYLIB,
    &frameworks::core_graphics::DYLIB,
    &frameworks::core_location::DYLIB,
    &frameworks::core_motion::DYLIB,
    &frameworks::foundation::DYLIB,
    &frameworks::game_kit::DYLIB,
    &frameworks::media_player::DYLIB,
    &frameworks::openal::DYLIB,
    &frameworks::opengles::DYLIB,
    &frameworks::store_kit::DYLIB,
    &frameworks::system_configuration::DYLIB,
    &frameworks::uikit::DYLIB,
];

#[cfg(test)]
mod tests {
    use crate::objc::ClassTemplate;

    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_classes() {
        let mut seen_classes = HashSet::new();

        for (class_name, template) in DYLIB_LIST
            .iter()
            .flat_map(|dylib| dylib.class_exports)
            .copied()
            .flatten()
        {
            if !seen_classes.insert(class_name) {
                panic!("Found duplicate class export {class_name}");
            }
            let ClassTemplate {
                class_methods,
                instance_methods,
                ..
            } = template;

            let mut seen_class_methods = HashSet::with_capacity(class_methods.len());

            for (method_name, _) in *class_methods {
                if !seen_class_methods.insert(method_name) {
                    panic!("Found duplicate class method {method_name} for class {class_name}")
                }
            }

            let mut seen_instance_methods = HashSet::with_capacity(instance_methods.len());

            for (method_name, _) in *instance_methods {
                if !seen_instance_methods.insert(method_name) {
                    panic!("Found duplicate instance method {method_name} for class {class_name}")
                }
            }
        }
    }

    #[test]
    fn no_duplicate_functions() {
        let mut seen = HashSet::new();

        for (function_name, _) in DYLIB_LIST
            .iter()
            .flat_map(|dylib| dylib.function_exports)
            .copied()
            .flatten()
        {
            if !seen.insert(function_name) {
                panic!("Found duplicate function export {function_name}");
            }
        }
    }

    #[test]
    fn no_duplicate_constants() {
        let mut seen = HashSet::new();

        for (constant_name, _) in DYLIB_LIST
            .iter()
            .flat_map(|dylib| dylib.constant_exports)
            .copied()
            .flatten()
        {
            if !seen.insert(constant_name) {
                panic!("Found duplicate constant export {constant_name}");
            }
        }
    }
}
