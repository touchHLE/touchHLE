# Building touchHLE

You need [git](https://git-scm.com/), [the Rust toolchain](https://www.rust-lang.org/tools/install), and your platform's standard C and C++ compilers.

First check out the git repo with `git clone`. Also make sure you get the submodules (`git submodule update --init` should be enough).

There is one special external dependency, Boost:

* On Windows, download it from <https://www.boost.org/users/download/> and extract the contents of the directory with a name like `boost_1_81_0` to `vendor/boost`.
* On other OSes, install Boost from your package manager. If you are on macOS and using [Homebrew](https://brew.sh/): `brew install boost`.

Then you just need to run `cargo run --release` (for a release build) or `cargo run` (for a debug build) to build and run touchHLE. On an underpowered, passively-cooled, 2-core laptop (2017 Retina MacBook), a clean release build takes a bit less than 9 minutes.

The `touchHLE_dylibs` and `touchHLE_fonts` directories contain files that the resulting binary will need at runtime, so you'll need to copy them if you want to distribute the result. You also should include the license files.

If you're building touchHLE for the purpose of contributing, you might want to generate HTML documentation with `cargo doc --workspace --no-deps --open`. The code has been extensively commented with `cargo doc` in mind.
