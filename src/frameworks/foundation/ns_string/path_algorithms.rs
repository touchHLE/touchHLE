/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Implementations of some path manipulation algorithms used by NSString
//! methods.
//!
//! These often have completely different behavior to the Rust path algorithms,
//! so while it's interesting to compare these with [crate::fs::GuestPath],
//! they shouldn't be merged.

/// Returns a tuple with the `stringByDeletingLastPathComponent` and
/// `lastPathComponent` values for a string, in that order.
pub fn split_last_path_component(path: &str) -> (&str, &str) {
    let path = {
        let without_trailing_slashes = path.trim_end_matches('/');
        if without_trailing_slashes.is_empty() && path.starts_with('/') {
            return ("/", "/");
        }
        without_trailing_slashes
    };

    if let Some((rest, last_path_component)) = path.rsplit_once('/') {
        let rest = if rest.is_empty() && path.starts_with('/') {
            "/"
        } else {
            rest
        };
        (rest, last_path_component)
    } else {
        ("", path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_split_last_path_component() {
        fn string_by_deleting_last_path_component(path: &str) -> &str {
            super::split_last_path_component(path).0
        }
        fn last_path_component(path: &str) -> &str {
            super::split_last_path_component(path).1
        }

        // These take inspiration from the examples from Apple's documentation,
        // which are a useful reference for how these methods should behave.

        assert_eq!(string_by_deleting_last_path_component("/a/b"), "/a");
        assert_eq!(string_by_deleting_last_path_component("/a/b/"), "/a");
        assert_eq!(string_by_deleting_last_path_component("/a/b///"), "/a");
        assert_eq!(string_by_deleting_last_path_component("/a/"), "/");
        assert_eq!(string_by_deleting_last_path_component("/a"), "/");
        assert_eq!(string_by_deleting_last_path_component("/"), "/");
        assert_eq!(string_by_deleting_last_path_component("a"), "");

        assert_eq!(last_path_component("/a/b"), "b");
        assert_eq!(last_path_component("/a/"), "a");
        assert_eq!(last_path_component("a//////"), "a");
        assert_eq!(last_path_component("/"), "/");
    }
}
