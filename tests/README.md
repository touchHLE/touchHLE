Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to an ARMv6 Mach-O binary and packaged into a bundle (`TestApp.app`) so that they can be run in the emulator like a normal iPhone OS app. The code in `integration.rs` lets them be run by `cargo test` (which also runs unit tests written in Rust).
