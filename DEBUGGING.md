# Debugging touchHLE

See also `BUILDING.md`.

## Logging

`src/log.rs` provides two logging macros, `log!()` and `log_dbg!()`. The former always prints a log message, whereas the latter only prints a message if the containing module is listed in `ENABLED_MODULES` in the same file.

## Debugging crashes in host code

The `RUST_BACKTRACE=1` environment variable is always helpful. You'll probably want a debug (not `--release`) build of touchHLE to get the best output.

## Debugging crashes in guest code

touchHLE will print the basic registers (r0-r13, SP, LR, PC) and a basic stack trace (using frame pointers) for the current thread when a panic occurs. To make sense of the result, you will probably want to open the app binary in Ghidra or another reverse-engineering tool.

### GDB Remote Serial Protocol server

For more complex cases, you can use the `--gdb=` command-line argument to start touchHLE in debugging mode, where it will provide a GDB Remote Serial Protocol server. You can then connect to touchHLE with GDB. (In theory LLDB also should work, but it doesn't.)

You'll need a version of GDB that supports ARMv6. On macOS, the Homebrew package for `gdb` is multi-architecture. If you're on Ubuntu, you might need the `gdb-multiarch` package (this hasn't been tested).

The basic set of steps is:

* Start touchHLE in debugging mode: `touchHLE --gdb=localhost:9001 'Some App.app'`.
* In a separate terminal window, start GDB: `gdb 'Some App.app/SomeApp'`. (You can omit the executable path, but this leaves GDB with no debug symbol info, [which is a worse experience](https://sourceware.org/bugzilla/show_bug.cgi?id=30234).) Then, inside GDB:
  * `set arch armv6`
  * `target remote localhost:9001`

You can make GDB connect immediately if you prefer: `gdb -ex 'set arch armv6' -ex 'target remote localhost:9001'`.

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

touchHLE only communicates with GDB while execution is paused. Beyond being paused when you initially connect, it is also paused when certain CPU errors occur, or after stepping (resuming execution for a single instruction). Breakpoints are a useful way to force execution to pause at convenient locations.

## Graphics debugging

[apitrace](https://apitrace.github.io/) is invaluable for figuring out OpenGL-related issues.

Outside the OpenGL realm, sometimes the most effective solution is dumping image data to a file. You can use Rust's `std::fs::write` for this. If you're a GIMP user, you might want to use it to open raw RGBA8 image data (easiest if the filename ends in `.data`), though there are probably better tools.
