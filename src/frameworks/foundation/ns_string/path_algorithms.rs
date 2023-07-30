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
//!
//! The examples in Apple's documentation for the corresponding NSString methods
//! are a useful reference for figuring out how the algorithm should work, and
//! as a source of inspiration for test cases.

pub fn trim_trailing_slashes(path: &str) -> &str {
    let without_trailing_slashes = path.trim_end_matches('/');
    if without_trailing_slashes.is_empty() && path.starts_with('/') {
        "/"
    } else {
        without_trailing_slashes
    }
}

/// Returns a tuple with the `stringByDeletingLastPathComponent` and
/// `lastPathComponent` values for a string, in that order.
pub fn split_last_path_component(path: &str) -> (&str, &str) {
    let path = trim_trailing_slashes(path);

    if path == "/" {
        ("/", "/")
    } else if let Some((rest, last_path_component)) = path.rsplit_once('/') {
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

/// Returns the `pathComponents` values for a string. Note that this behaves
/// differently to `lastPathComponent`/`stringByDeletingLastPathComponent`.
pub fn split_path_components(path: &str) -> Vec<&str> {
    let mut components = Vec::new();

    let path = if let Some(path) = path.strip_prefix('/') {
        components.push("/");
        path
    } else {
        path
    };
    let (path, trailing_slash) = if let Some(path) = path.strip_suffix('/') {
        (path, true)
    } else {
        (path, false)
    };

    for component in path.split('/') {
        if component.is_empty() {
            continue;
        }
        components.push(component);
    }

    if trailing_slash {
        components.push("/");
    }

    components
}

/// Returns a tuple with the `stringByDeletingPathExtension` and
/// `pathExtension` values for a string, in that order.
pub fn split_path_extension(path: &str) -> (&str, &str) {
    let path = trim_trailing_slashes(path);

    let (_, last_path_component) = split_last_path_component(path);
    // A filename beginning with '.' is not considered as an extension.
    if last_path_component.contains('.')
        && (!last_path_component.starts_with('.') || last_path_component[1..].contains('.'))
    {
        path.rsplit_once('.').unwrap()
    } else {
        // No extension.
        (path, "")
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

    #[test]
    fn test_split_path_components() {
        use super::split_path_components;

        assert_eq!(&split_path_components("a/b"), &["a", "b"]);
        assert_eq!(&split_path_components("/a/b"), &["/", "a", "b"]);
        assert_eq!(&split_path_components("a/b/"), &["a", "b", "/"]);

        assert_eq!(&split_path_components("a///b"), &["a", "b"]);
        assert_eq!(&split_path_components("///a/b"), &["/", "a", "b"]);
        assert_eq!(&split_path_components("a/b///"), &["a", "b", "/"]);

        assert!(split_path_components("").is_empty());

        assert_eq!(&split_path_components("/"), &["/"]);

        // Weird edge-case discovered when testing macOS's implementation.
        // Bug compatibility?
        assert_eq!(&split_path_components("//"), &["/", "/"]);
        assert_eq!(&split_path_components("///"), &["/", "/"]);
    }

    #[test]
    fn test_split_path_extension() {
        fn string_by_deleting_path_extension(path: &str) -> &str {
            super::split_path_extension(path).0
        }
        fn path_extension(path: &str) -> &str {
            super::split_path_extension(path).1
        }

        assert_eq!(string_by_deleting_path_extension("/a/b.png"), "/a/b");
        assert_eq!(string_by_deleting_path_extension("/a/"), "/a");
        assert_eq!(string_by_deleting_path_extension("a.png/"), "a");
        assert_eq!(string_by_deleting_path_extension("a..png"), "a.");
        assert_eq!(string_by_deleting_path_extension("a.gif.png"), "a.gif");
        assert_eq!(string_by_deleting_path_extension("~/.ssh"), "~/.ssh");
        assert_eq!(string_by_deleting_path_extension(".a.png"), ".a");
        assert_eq!(string_by_deleting_path_extension("/"), "/");

        assert_eq!(path_extension("/a/b.png"), "png");
        assert_eq!(path_extension(".a.png"), "png");
        assert_eq!(path_extension("~/.ssh"), "");
        assert_eq!(path_extension("/a/b"), "");
        assert_eq!(path_extension("/a/"), "");
        assert_eq!(path_extension("/a/a..png"), "png");
    }
}
