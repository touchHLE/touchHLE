//! The Foundation framework.
//!
//! A concept that Foundation really likes is "class clusters": abstract classes
//! with private concrete implementations. Apple has their own explanation of it
//! in [Cocoa Core Competencies](https://developer.apple.com/library/archive/documentation/General/Conceptual/DevPedia-CocoaCore/ClassCluster.html).
//! Being aware of this concept will make common types like `NSArray` and
//! `NSString` easier to understand.

pub mod ns_array;
pub mod ns_autorelease_pool;
pub mod ns_coder;
pub mod ns_keyed_unarchiver;
pub mod ns_object;
pub mod ns_string;

#[derive(Default)]
pub struct State {
    ns_string: ns_string::State,
}
