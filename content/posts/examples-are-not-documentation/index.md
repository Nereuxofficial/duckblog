+++  
title = "Examples are not Documentation"  
date = "2024-05-02"  
tags = ["rust", "documentation"]  
keywords = ["rust", "documentation", "iced"]
description = "My thoughts on the state of Rust documentation and how it can be improved."
showFullContent = false  
draft = false  
+++
A while ago friends of mine tried writing a GUI using [iced](https://github.com/iced-rs/iced). Iced is a really cool GUI framework no doubt, [System76 is even making a desktop environment using it](https://blog.system76.com/post/locked-and-loaded-with-new-cosmic-de-updates) which I will absolutely try once it's ready. Inspired by the [ELM Architecture](https://guide.elm-lang.org/architecture/) it seems like the most promising GUI framework programmable (primarily) via Rust, however it's [book](https://book.iced.rs/), consisting mostly of TODOs, could use some love. (Since i started writing this post the book has started getting a few really good pages, but it still has a lot of TODOs)

As a Rustacean I have gotten adjusted to the notion of trying out the examples and reading through the source code, but while automatically compiled and run examples are great, they don't replace good documentation and often fail to explain the underlying concepts leading to frustration for novice users. Projects and Guides using the software are also nice, but sometimes use an [older version of the framework](https://github.com/iced-rs/awesome-iced). This is not a criticism of iced specifically, but more a lack I've noticed with many Rust projects. 

Examples get users familiar with the subject matter up to speed really quickly but unfamiliar users are often left in the dark with to them magical functions, structs and maybe even macros(where new users may not know how to [expand them](https://github.com/dtolnay/cargo-expand)). Thankfully in most cases it is Rust most of the way down and in most cases the Rust code is readable pretty easily, so figuring out the function of something is relatively easy, but even in that case good documentation gives users of a library more orientation in the given function.
# How can this be fixed?
Of course for many projects this is not a priority. Many crates are very much a Work-In-Progress and breaking changes are to be expected. As such most documentation would have to be rewritten and quickly outdated, although I still find outdated documentation better than none, since it can be easier to figure out what has changed. But if you do want to improve documentation there are many options that makes the life of Rust users and developers easier. Besides the `///` to [document functions, components and modules in Markdown](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html) these are things that stuck out to me.

## Links
Links are a pretty great way to provide more context, while allowing users who may know, say a [Skip List](https://en.wikipedia.org/wiki/Skip_list) to read ahead.
## Books
I think [the async book](https://github.com/rust-lang/async-book), which is still sadly unfinished, makes a good example of how to write a [mdbook](https://rust-lang.github.io/mdBook/)but still have CI for your examples, by writing your code in Rust files and including them in the mdbook like this:
`` ```rust,edition2018 ``
`{{#include ../../examples/06_04_spawning/src/lib.rs:example}}`
`` ``` ``

## Sharing your projects
If you made a project that you think could be useful to somebody: Share it and document your findings and difficulties and quirks you encountered, so other users may learn from your mistakes.
## Examples of how to use functions
I think examples of how to use functions are really great, they show users in their IDE how to use the thing I am working with. Bonus Points if they show what assumptions about them are wrong. As for keeping them updated: [cargo test runs them by default](https://doc.rust-lang.org/cargo/commands/cargo-test.html#documentation-tests) and breaking these tests may remind you to update the now-outdated documentation as well.
![[images/filter_map_documentation.png]]

## What can you do?
That's all nice and good. Library authors should be aware that good documentation often saves users a lot of trouble(and maybe themselves some Issues). But what can you do as a user of said library? 

If you see something that you don't understand(and have already researched), try to figure out how it works by reading through the code and maybe make an issue(the issue tracker will of course also help other users having similar problems). When you understand it, try to write documentation from a user's perspective(bonus points for examples and showing which assumptions are wrong). This can also massively help the maintainers, since writing documentation from a user's point of view may be really difficult. Then open a Pull Request and the author will most likely accept it, as they are almost always happy for some help. Be sure to read contribution guidelines though and if the maintainers want you to, report an issue first, to see whether documentation is wanted.

That being said i plan to write some posts about iced in the near future for some Linux window manager projects I have in mind. I hope to see you then!