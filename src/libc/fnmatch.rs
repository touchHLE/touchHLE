/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `fnmatch.h`
//! Match a filename or pathname against a shell-style pattern.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::ConstPtr;
use crate::Environment;
use std::collections::HashMap;

const FNM_NOMATCH: i32 = 1;

/// Inner helper function to match string `a` against pattern `b`.
/// `a_end` and `b_end` are suffixes length of the string and pattern
/// respectively. `mem` is used for memoization.
/// Only '*' wildcard is supported for the moment.
/// TODO: extend matching logic and generalize for non-ASCII cases
fn fnmatch_inner(
    a: &[u8],
    a_end: usize,
    b: &[u8],
    b_end: usize,
    mem: &mut HashMap<(usize, usize), bool>,
) -> bool {
    // TODO: if you feel extra fancy, try to rewrite using arrays (or vectors)
    // instead of memoizing with a map (iterative DP).
    if let Some(&val) = mem.get(&(a_end, b_end)) {
        return val;
    }
    let res = if b_end == 0 {
        // Empty pattern matches empty string only.
        // Note: it isn't true other way around! Think empty string and
        // "*" pattern
        a_end == 0
    } else if b[b_end - 1] == b'*' {
        // Iterate over all possible matches;
        // If we found at least one match, we don't need to continue
        (0..=a_end).any(|i| fnmatch_inner(a, i, b, b_end - 1, mem))
    } else if a_end == 0 {
        false
    } else {
        a[a_end - 1] == b[b_end - 1] && fnmatch_inner(a, a_end - 1, b, b_end - 1, mem)
    };
    log_dbg!("fnmatch_inner({a_end},{b_end}) -> {res}");
    mem.insert((a_end, b_end), res);
    res
}

pub(super) fn fnmatch(
    env: &mut Environment,
    pattern: ConstPtr<u8>,
    string: ConstPtr<u8>,
    flags: i32,
) -> i32 {
    let pattern_str = env.mem.cstr_at_utf8(pattern).unwrap();
    log_dbg!(
        "fnmatch({}, {:?}, {})",
        pattern_str,
        env.mem.cstr_at_utf8(string),
        flags
    );

    assert!(!pattern_str.contains('\\')); // TODO
    assert!(!pattern_str.contains('?') && !pattern_str.contains('[')); // TODO

    assert_eq!(flags, 0); // TODO
    let a = env.mem.cstr_at(string);
    let b = env.mem.cstr_at(pattern);

    let mut mem = HashMap::new();
    if fnmatch_inner(a, a.len(), b, b.len(), &mut mem) {
        0 // there is a match
    } else {
        FNM_NOMATCH
    }
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(fnmatch(_, _, _))];

#[cfg(test)]
mod fnmatch_tests {
    use super::*;

    #[test]
    fn fnmatch_inner_tests() {
        fn test_helper(a: &str, b: &str) -> bool {
            let mut mem = HashMap::new();
            fnmatch_inner(a.as_bytes(), a.len(), b.as_bytes(), b.len(), &mut mem)
        }

        assert!(test_helper("", ""));
        assert!(test_helper("", "*"));
        assert!(!test_helper("ab", ""));
        assert!(test_helper("ab", "ab"));
        assert!(!test_helper("ab", "ad"));
        assert!(test_helper("ab", "*"));
        assert!(test_helper("abc", "*c"));
        assert!(!test_helper("abc", "c*"));
        assert!(!test_helper("abc", "*a"));
        assert!(test_helper("abc", "a*"));
        assert!(test_helper("abcdkj", "a*d*"));
        assert!(!test_helper("abcdkj", "d*j"));
        assert!(test_helper("abcdkj", "ab***cdk****j"));
    }
}
