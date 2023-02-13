+++
title = "Rust extended operations"
date = "2022-10-09T21:31:55+02:00"
tags = ["rust", "programming"]
keywords = ["rust"]
description = "saturating_add() and more in Rust"
showFullContent = false
+++
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