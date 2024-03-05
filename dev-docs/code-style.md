# touchHLE coding style guide

This guide exists to give you some idea of what code in touchHLE should look like, and where possible, why. It should help you to write code that will pass code review, and help you to review others' code.

## touchHLE's unique requirements

touchHLE is a bridge between worlds: Objective-C versus Rust, guest code versus host code, 32-bit versus 64-bit, iPhone OS-exclusive APIs versus cross-platform open-source stuff, etc. This is true even for components written purely in one language. The coding style is necessarily a compromise between these worlds.

## Code formatting

Rust code is formatted with the standard Rust style and `rustfmt` can be used to reformat it. C and C++ code is formatted with the default style for `clang-format`. `dev-scripts/format.sh` will run both formatters.

Unfortunately, **`rustfmt` does not understand touchHLE's Objective-C macros**, so code inside `objc_classes!` and use of `msg!`, `msg_class!` and `msg_super!` must be manually formatted at the moment.

## Comments

touchHLE does not use the `/* */` syntax for multiple-line comments, with the sole exception of the “This Source Code Form is subject to […]” license header, which should be the first comment in every file.

`//` syntax should be used for a short comment at the end of a line, a comment that has its own line, or for one line out of many for a multiple-line comment. Lines containing `//` comments must not exceed 80 characters, including whitespace before the `//`. This rule is enforced by `dev-script/lint.sh`.

`/* */` can be used for tiny in-line comments to indicate the names of parameters when calling a function, or to comment on their values. This is often done for boolean parameters and others which don't have an obvious meaning otherwise, e.g. `do_something(/* asynchronously: */ true)`.

## Naming things

There are many places in code where you have to give something a name:

```rust
fn foo_bar() {}         // function named foo_bar
const FOO_BAR: i32 = 1; // constant named FOO_BAR
struct FooBar {}        // struct named FooBar
// …
```

touchHLE has two main approaches to naming things:

* touchHLE implements many frameworks and libraries that can be used by the guest app (e.g. UIKit, OpenGL ES 1.1, the C standard library, and the Objective-C runtime). These frameworks/libraries have APIs and ABIs that define functions, constants, types, classes, methods and so on, and all these things have names. Let's call these “external names”. Where possible and appropriate, touchHLE's implementations of these will have the same name as in the original API/ABI, and in their original forms (if the original is `kNSFooBar`, it won't be renamed to Rust-style `K_NS_FOO_BAR`, etc.).
* touchHLE internal code that is not _directly_ exposed to the guest app should use original names, to avoid [copyright concerns](../CONTRIBUTING.md#copyright-and-reverse-engineering), and generally follow the [naming conventions from the Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html). Let's call these “internal names”.

It's not always clear which rule applies though, so here's some more specific guidelines:

* Names that are part of the ABI must match external names exactly, otherwise touchHLE won't work. The main examples of these are the strings used in `FunctionExports`, `ConstantExports` and `ClassExports` lists, and Objective-C class names and selectors.
  * If you have no choice but to expose a _touchHLE-specific_ internal detail through the ABI, don't pick a name that looks like it might be external. Use a prefix like `_touchHLE_` to make it clear that the name originates from touchHLE, and to prevent potential conflicts with names coming from the guest app.
* Names of types and `#define`/enum-like constants (as opposed to `static const`-like/`ConstantExports` constants) are _not_ part of the ABI, but generally touchHLE will nonetheless use the API's name for it in its original form, in order to simplify cross-referencing of code with external documentation. For example, the type `UILineBreakMode` and its associated constants have the same names in [Apple's documentation](https://developer.apple.com/documentation/uikit/uilinebreakmode) and in [touchHLE's implementation](https://github.com/touchHLE/touchHLE/blob/d70b90b2de50b11595110e0b04f6aeff6a570d11/src/frameworks/uikit/ui_font.rs#L39-L52). Note that due to [copyright concerns](../CONTRIBUTING.md#copyright-and-reverse-engineering) you must not copy the names of non-public API implementation details, including internal macros and types in C headers.
* When implementing a C/Objective-C function, the name of the Rust function (`fn some_function_name_here(env: &mut Environment, …)`) used to implement it is also not part of the ABI, but in almost all cases the `export_c_func!` macro is used to export it by the same name, so it's easiest to preserve the external name in its original form, and this is also the recommendation when this macro is not used. For example, `UIGraphicsPushContext` has the same name in [Apple's documentation](https://developer.apple.com/documentation/uikit/1623921-uigraphicspushcontext?language=objc) and in [touchHLE's implementation](https://github.com/touchHLE/touchHLE/blob/d70b90b2de50b11595110e0b04f6aeff6a570d11/src/frameworks/uikit/ui_graphics.rs#L20-L46).
* The names of parameters for C functions and Objective-C methods (not to be confused with any relevant parts of the _selector_) are neither part of the API nor of the ABI, so in general the external names do not need to be preserved, and it might be unwise to mirror them too closely due to [copyright concerns](../CONTRIBUTING.md#copyright-and-reverse-engineering). They should follow Rust naming conventions.
* The names of Rust modules/files are a bit of an in-between. They are internal names but generally take some inspiration from the external names of the things they're for, so they follow the Rust module naming convention more or less (e.g. `UIKit` becomes `ui_kit`).

Note that preserving original forms may make the Rust compiler upset at you. Use of `#[allow(non_camel_case_types)]`, `#[allow(non_upper_case_globals)]`, `#[allow(non_snake_case)]` and `#[allow(clippy::upper_case_acronyms)]` where necessary is encouraged.
