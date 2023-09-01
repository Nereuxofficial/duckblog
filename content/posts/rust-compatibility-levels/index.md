+++
title = "Rust Library Compatibility Levels"
date = "2023-09-01T21:31:55+02:00"
tags = ["rust", "programming", "library", "crate", "compatibility"]
keywords = ["rust", "programming"]
description = "From microcontrollers to full-blown Linux systems, Rust has you covered!"
draft = false
showFullContent = false
+++
Rust is a general-purpose language, as such it can be used in a variety of environments going from bare-metal microcontrollers([as previously explored here](https://nereux.blog/posts/esp32-ws2812-dino-light-2/)) to Linux systems with many Layers of abstractions.
As such the libraries developed for it sometimes may or may not work in different environments, depending on what they use.


So let's go **lower** and lower in terms of features and abstractions, but **higher** in terms of compatibility:
- [**std**](https://doc.rust-lang.org/std/)(PCs, some microcontrollers)
	- 'std' stands for standard library and is Rusts default library offering many useful data structures and Abstractions going from [Strings](https://doc.rust-lang.org/std/string/index.html) to [TCPStreams](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
	- The default Rust run mode(If you've ever written "Hello World" in Rust you used it)
- ([**WASIX**](https://wasix.org/))(Superset of WASI, personally I remain sceptical of this one, as it replicates Unix APIs and it may repeat mistakes of the past)
	- More Unix-like APIs which can be built upon, which makes it possible to run cURL for example
- [**WASI**](https://github.com/WebAssembly/WASI)(Extended Webassembly, adding more APIs)
  - This enables many [System calls](https://en.wikipedia.org/wiki/System_call), enabling more complex programs to run. [wasmtime](https://github.com/bytecodealliance/wasmtime) is a runtime supporting this. These runtimes can function as lightweight container alternatives
- [**WASM**](https://www.rust-lang.org/what/wasm)(Webassembly, most commonly seen in Browsers)
  - Most commonly used for games in browsers, but it's a binary format which is very portable, which explains the browser use case
  - You can even use network operations!(Threading for example is limited though)
- [**no-std + alloc**](https://docs.rust-embedded.org/book/intro/no-std.html)(Many Microcontrollers + PC)
	- like no-std but with allocations, which helps a lot for e.g. Vectors
- [**no-std**](https://docs.rust-embedded.org/book/intro/no-std.html)(Runs even on the smallest of microcontrollers, when you're writing bare-metal code with minimal abstractions)
	- `#[no_std]` ensures no standard library(except for the platform-agnostic [core](https://doc.rust-lang.org/core/) part) is used. Crates have to be marked as `#![no_std]` to be used in this mode.
	- no heap allocations of any kind(like String, Vec etc.)

Different libraries support different levels of compatibility, and often allow running on `no_std` via no-default-features, such as [serde](https://github.com/serde-rs/serde) or [serde_json](https://github.com/serde-rs/json), both of which support no-std with some restrictions.

And it has to be said that no library author is obliged to support any of these levels, but it's nice if they do, as I found out [while playing around with ESP32 microcontrollers](https://nereux.blog/posts/esp32-ws2812-dino-light/). And if you are designing a library, it might be worth considering supporting some of these layers, as it makes your library more versatile and thus more useful, and also mentioning it clearly in the README.

Rust also has a [list of targets](https://doc.rust-lang.org/rustc/platform-support.html) along with what they support.

I hope this was helpful to you, if you have any questions or suggestions feel free to [contact me](https://nereux.blog/contact/).
And if you want to support me and my work, you can do so [here](https://github.com/sponsors/Nereuxofficial).