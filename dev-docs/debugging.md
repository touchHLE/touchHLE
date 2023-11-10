# Debugging touchHLE

See also `BUILDING.md`.

## Logging

`src/log.rs` provides two logging macros, `log!()` and `log_dbg!()`. The former always prints a log message, whereas the latter only prints a message if the containing module is listed in `ENABLED_MODULES` in the same file.

Some modules you might want to enable:

* The combination of `touchHLE::abi` and `touchHLE::dyld` gives you a trace of almost all guest-to-host calls, among other things
* `touchHLE::mem` logs memory allocations and deallocations

## Debugging crashes in host code

The `RUST_BACKTRACE=1` environment variable is always helpful. You'll probably want a debug (not `--release`) build of touchHLE to get the best output.

## Debugging crashes in guest code

touchHLE will print the basic registers (r0-r13, SP, LR, PC) and a basic stack trace (using frame pointers) for the current thread when a panic occurs. To make sense of the result, you will probably want to open the app binary in Ghidra or another reverse-engineering tool.

### GDB Remote Serial Protocol server

For more complex cases, you can use the `--gdb=` command-line argument to start touchHLE in debugging mode, where it will provide a GDB Remote Serial Protocol server. You can then connect to touchHLE with GDB. (In theory LLDB also should work, but it doesn't.)

A quick word of warning: this will not be the GDB experience you may be used to when writing C/C++ code and compiling it in debug mode. The GDB support was added to help with debugging apps for which we don't have symbols, let alone DWARF info or source code. GDB when connected to touchHLE will not know about local variables or even stack frames! You'll need to know instruction addresses and register numbers. As such, having the binary open in a tool like Ghidra while debugging is practically mandatory.

Anyway, you'll need a version of GDB that supports ARMv6. On macOS, the Homebrew package for `gdb` is multi-architecture. If you're on Ubuntu, you might need the `gdb-multiarch` package (this hasn't been tested).

The basic set of steps is:

* Start touchHLE in debugging mode: `touchHLE --gdb=localhost:9001 'Some App.app'`.
* In a separate terminal window, start GDB: `gdb 'Some App.app/SomeApp'`. (You can omit the executable path, but this leaves GDB with no debug symbol info, [which may be a worse experience](https://sourceware.org/bugzilla/show_bug.cgi?id=30234).) Then, inside GDB, run `target remote localhost:9001` to connect to touchHLE.

If you prefer for GDB to connect immediately: `gdb 'Some App.app/SomeApp' -ex 'target remote localhost:9001'`.

When GDB first connects, CPU execution is paused and none of the guest app's code has been run yet. While execution is paused, touchHLE allows GDB to:

* Read and write registers
* Read and write memory
* Resume execution, either indefinitely or for a single instruction
* Kill the emulated app (this just makes touchHLE crash)

GDB provides various services on top of this, for example:

* `break *0x1000` sets a breakpoint
* `info registers` shows the content of registers
* `backtrace` shows a backtrace (though touchHLE's own may be better)
* `print *(float*)0x2000` evaluates a simple C-like expression
* `layout asm` opens a disassembly view
* `kill` will make touchHLE crash
* `step` resumes execution for a single instruction
* `continue` resumes execution indefinitely

Beware that iPhone OS apps often contain a mix of Thumb functions and normal Arm functions. GDB usually won't know which kind of function it's dealing with:

* When no symbols are available, GDB will assume an address is Arm code by default. You can use `set arm fallback-mode` to change this assumption.
* When full symbols are available, GDB seems to assume symbols are for Arm functions [even when they aren't](https://sourceware.org/bugzilla/show_bug.cgi?id=30386). You can use `set arm force-mode` to override this.

GDB seems to [mostly](https://sourceware.org/bugzilla/show_bug.cgi?id=30385) understand the convention of setting the lower bit of the address to 1 to indicate a Thumb function, and in any case setting an Arm breakpoint in Thumb code (not vice-versa) usually works, so you usually only need to worry about this when disassembling things.

touchHLE only communicates with GDB while execution is paused. Beyond being paused when you initially connect, it is also paused when certain CPU errors occur, or after stepping (resuming execution for a single instruction). Breakpoints are a useful way to force execution to pause at convenient locations. Another option is to press the F12 key while you have the touchHLE window in focus, which will make touchHLE pause during the next NSRunLoop iteration. If the app fails to return to the NSRunLoop then this won't be useful.

## Graphics debugging

[apitrace](https://apitrace.github.io/) is invaluable for figuring out OpenGL-related issues.

More generally, and especially Outside the OpenGL realm, sometimes the most effective solution is dumping image data to a file. There's some functions in [`crate::debug`](../src/debug.rs) that might be useful for this. Don't forget that you can also use Rust's `std::fs::write` if necessary. GIMP and some other tools can read raw pixel data (easiest if the filename ends in `.data`).
