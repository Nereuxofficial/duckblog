---
title: "Rust extended operations"
date: "2022-10-09"
tags: ["rust", "programming"]
draft: true
description: "saturating_add() and more in Rust"
url: /posts/rust_extended_operations
---

You may know the default Rust operations like +, - and so on. But there are more functions that
may suit you better in rare cases.
In the following I call the default operations the "normal" operations and the argument `rhs`(`function(rhs)`).

| Function    | returns     | notes                                                                                            |
|-------------|-------------|--------------------------------------------------------------------------------------------------|
| normal      | `T`         |                                                                                                  |
| try         | `Result<T>` | *Computes self + rhs, returning an Error if an overflow occured*                                 |
| checked     | `Option<T>` | *Computes self + rhs, returning None if an overflow occurred*                                    |
| unchecked   | `T`         | *Computes assuming an over/underflow cannot occur(Results in undefined behaviour upon overflow)* |
| saturating  | `T`         | *Computes self + rhs, saturating at the numeric bounds instead of overflowing*                   |
| wrapping    | `T`         | *"wraps" around the boundary, same as with modulo T::max*                                        |
| overflowing | `T`         | *Overflows instead of panicking upon an overflow*                                                |