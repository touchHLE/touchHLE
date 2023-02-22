# Code-level (HLE)
- **ipasim** (2017) is a high level emulator written in C++ that uses Microsoft's winObjC to recompile 64-bit iOS apps into apps in live. It doesn't support much apps except for really basic ones. It was more focused on apps then games. [Source code](https://github.com/ipasimulator/ipasim) - [Project page](https://janjones.me/projects/ipasim/) - [Paper](https://github.com/ipasimulator/ipasim/blob/master/docs/thesis/thesis.pdf) - [Poster](https://github.com/ipasimulator/ipasim/blob/master/docs/thesis/poster.pdf)
- **unidbg** (2020) is a developer library that allows emulating Android binaries, however "expiramental" iOS support was added some time in 2021. [Source code](https://github.com/zhkl0228/unidbg)
# Middle-level or other
- **macOS 11+** (2020-present) on Apple Silicon architexures, which are are not virtualisable yet can run iOS apps, altough you need a few workarounds to sideload some apps. Emulation is basically perfect. [Website](http://apple.com/macos)
- **Cycada** (2014), formally known as Cider is an unreleased reasearch project made by a few folks at Columbia that ran iOS 5.1.1 and experimentally iOS 6 apps at a high, but not perfect quality and compatibility. It is based on pirated iOS libraries. It is seriously not reccomended to initiate contact with the developers of the project, as they never planned on releasing it and want people to use their paper to reproduce it with "significant effort". You may try to recreate Cycada on your own, provided that you know the internals of Android, iOS, XNU, and Linux.
# Device-level (emulation)

# Scams
Basically any other tool that claims to emulate iOS on any other platform is most likely a scam. It either is completely fake and a malware or just is an interface that looks like iOS. One of the most popular scams is called iPadian, which is a paid iOS simulator.
