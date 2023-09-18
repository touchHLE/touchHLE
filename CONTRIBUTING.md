# Contributing to touchHLE

Please also read the [code of conduct](CODE_OF_CONDUCT.md).

## Issues

Please bear in mind that there are infinitely many apps that do not work in touchHLE right now, so please don't open issues about apps that aren't in [the supported list](APP_SUPPORT.md), unless you've got it partially working (e.g. loaded up to the menu but the main game doesn't work). The fact that an app's splash screen (Default.png) shows up doesn't mean it's partially working.

## Code contributions

[BUILDING.md](BUILDING.md) and [DEBUGGING.md](DEBUGGING.md) might be helpful while you're working on a contribution.

Please run `cargo fmt` and `cargo clippy` on your changes before committing. For the handful of C and C++ files, please use `clang-format -i` to format them.

You should also run `cargo test`. [Building the integration tests requires downloading LLVM](tests/BUILDING.md), so it's understandable if you want to skip them (`cargo test -- --skip run_test_app`) and let the GitHub Actions CI catch any issues when you submit your pull request. Alternatively, you can download a pre-built version of the integration tests app (TestApp) from GitHub Actions CI and run it in touchHLE.

If you're going to open a pull request with non-trivial changes, please talk to us first so we can figure out if we're likely to accept them. It would be a shame if your effort had to be wasted.

### Copyright and reverse engineering

(Please also read the copyright rules in the code of conduct.)

⚠️ Be **very** careful about copyright. To put it simply: **don't contribute if you've seen code you shouldn't have seen, don't copy code that isn't yours to copy, and especially don't _secretly_ copy and pretend you didn't**. Any infringement of Apple or other copyrights could threaten the foundations of the project, and the livelihoods of current contributors. **If in doubt, don't do it**, but in particular:

* ⚠️ When implementing an API, rely firstly and primarily on public documentation.
* ⚠️ Do not under any circumstances look at or rely on _leaked_ code, documentation, tools, etc. Material being available somewhere does not mean it is open-source.
* ⚠️ Do not disassemble or decompile components of iPhone OS or other Apple platforms. If you can't figure out how else you would find out how an API should behave, please just don't try to implement it.
* ⚠️ Looking at header files is occasionally necessary, but it should not be your first resort, and you must only use them as a source of simple facts (e.g. what value does a constant have, what type does a type alias resolve to). Do not copy their layout and organization. Do not copy anything you do not need to. Except where the name is part of the ABI or public API, do not copy names.
* ⚠️ Bear in mind that open-source code is still covered by copyright, and so the same caution applies to consulting open-source code. Especially try to avoid looking at the implementation files, unless there is no other option, and do not copy algorithms. (Note however that if it's under a compatible license, we may be able to bring the open-source code into the project _under that license, as a dependency_.)
* ⚠️ If you work or have worked at Apple, or NeXT, or various other organisations, then you may have seen the proprietary source code used in components of iPhone OS. If that's the case, please do not contribute to this project.
* ⚠️ If your employment contract or applicable law in your country means that you don't own the copyright on code you want to contribute to this project, or if for some other reason you may need permission from your employer to contribute to this project: please do obtain that permission before contributing.
