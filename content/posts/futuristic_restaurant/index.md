+++
title = "Futuristic Restaurant"
description = "A simple analogy for how futures work in Rust"
date = "2023-07-10"
tags= ["futures", "async", "tokio", "rust"]
keywords= ["rust", "async", "futures"]
draft = true
+++
Imagine going to a restaurant. After a few minutes the waiter comes to you and asks you whether you've decided what to order yet. You think a bit and respond with "I am not ready to order yet" and after a while he asks you again and the you say after further thinking: "Here's my order: A plate of whimsical unicorn-shaped spaghetti!". That is, in a very simplified way, how futures work in Rust.

## Code
Now we implement the example. We set up the project with these commands:
```bash
# Make a new binary project
cargo new futuristic_restaurant
cd futuristic_restaurant
# For logging
cargo add tracing tracing-subscriber
# Our async runtime. We're using tokio since it's the most commonly used one, but async-std works too of course
cargo add tokio --features full
```
And write this to our `main.rs`:
```rust
use tracing::*;

// This is a macro, that spawns tokio's executor and allows us to write our main function asynchronously.
// DO NOT WRITE THIS OVER EVERY async fn like i did many years ago
#[tokio::main]
async fn main(){
	// Initialize Logging
    tracing_subscriber::fmt::init();
    info!("Hello, you futuristic restaurant!");
}
```
And then run it:
```bash
export RUST_LOG=debug # we can choose a logging level of: trace, debug, info, error
cargo r -q
2023-06-08T19:34:45.950778Z  INFO futuristic_restaurant: Hello, you futuristic restaurant!
```
And onto implementing our guest, who in async speak is a Future. A Future is the trait that is core to async in Rust. So let's make a struct Guest and implement the trait `Future` for it. The Future trait contains an Output, being our return type when our future is ready and the function `poll`:
```rust
use std::{future::Future, task::Context};
use tokio::time::Instant;
struct Guest{
    time_ready: Instant
}
impl Guest{
    // We don't return Self here because we want to want to deal directly with the future 
    fn new_waiting_guest() -> impl Future<Output = String> {
        Guest {
            time_ready: Instant::now() + std::time::Duration::from_secs(1),
        }
    }
}
impl Future for Guest {
    type Output = String;
    // Pin is a pointer that can't be moved in memory, ensuring that the future is always in the same place
    // Context is the context of the future, that currently only provides a waker
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
	    // In reality we could do some work here
        if Instant::now() >= self.time_ready {
            info!("Here's my order:");
            // Our returned Poll can either be Ready with data in it
            std::task::Poll::Ready("A plate of whimsical unicorn-shaped spaghetti!".to_string())
        } else {
            // This tells the executor that the future should be polled again immediately, which is not efficient but fine for demonstration purposes
            cx.waker().wake_by_ref();
            // Or Pending, which tells the executor that the future is not ready yet
            std::task::Poll::Pending
        }
    }
}
```
And you can imagine the poll method as being the asking if the guest is ready to order yet.  Now let's create our first guest and we'll make him order in one second because we don't have patience.
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Hello, you futuristic restaurant!");
    // Finally! Our first guest!
    Guest::new_waiting_guest();
}
```
So now our guest should....
```bash
cargo r -q
warning: unused implementer of `Future` that must be used
  --> src/main.rs:11:5
   |
11 |     Guest::new_waiting_guest();
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: futures do nothing unless you `.await` or poll them
   = note: `#[warn(unused_must_use)]` on by default

2023-06-09T21:11:40.951144Z  INFO futuristic_restaurant: Hello, you futuristic restaurant!
```
Oh no, where has our Future gone? 

%Coolduck says%
It was dropped. You can see in the Output that that guest nothing unless you either poll it or await it(where the executor polls it until it is ready). In fact that is the only way for a Future to do anything. With poll you ask the function whether it is ready(like the guest being ready to order) and it returns the Poll. Awaiting polls something as long as it is ready.
%coolduck%

And we can print when he leaves using this:
```rust
/// The Drop Trait is typically implemented for Data that needs to do extra work on Cleanup(like many heap allocated things like Vec, Box etc.)
/// It is called automatically when an object goes out of scope, a core part of RAII(Resource Acquisition Is Initialization)
impl Drop for Guest{
	fn drop(&mut self) {
        println!("I'm leaving... I've been sitting here for hours");
        // We don't need to do more because the compiler does the rest for us
    }
}
```
Alright! Let's await our Guest.
```rust
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Hello, you futuristic restaurant!");
    let guest = Guest::new_waiting_guest();
    // Note that we need to be in an async function to await something
    println!("{}", guest.await);
}
```
```bash
cargo r -q
2023-06-10T16:05:21.679489Z  INFO futuristic_restaurant: Hello, you futuristic restaurant!
2023-06-10T16:05:22.679536Z  INFO futuristic_restaurant: Here's my order:
A plate of whimsical unicorn-shaped spaghetti!
```
And you may have spotted that we could do this just as well in a blocking fashion, which absolutely true. But as we add more guests the overhead of blocking is much higher. Async is especially useful in cases where we're dealing with IO(especially networking). For example when you have a webserver and have to serve many requests where some may be on a really slow connection it is really inefficient to spawn actual threads for every one of them especially when they are not doing something most of the time. The executor(in our case tokio) then suspends our tasks and polls other tasks depending on when they get a wake.

A good example of this is implementing a webserver. You can either [do it in a blocking manner](https://doc.rust-lang.org/book/ch20-02-multithreaded.html)(where it would be pretty much impossible to reach 1000 requests on one server) or do it asynchronously, where we spawn an async task for every connection and can handle many connections asynchronously, some even on a single thread(Caution: multiple threads = concurrent, asynchronous = not actually concurrent).

Here is a small example of how to do this with Actix-web(currently the #2 web framework in Rust):
```bash
cargo new single_threaded_webserver
cargo add tokio --features full
cargo add actix_web
```

```rust
use actix_web::{get, App, HttpServer, Responder};

#[get("/")]
async fn greet() -> impl Responder{
    format!("Hello futuristic restaurant!")
}

// With flavor we can choose how many worker threads we want
#[tokio::main(flavor="current_thread")]
async fn main() {
    HttpServer::new(||{
        App::new().service(greet)
    })
    .bind(("127.0.0.1", 8080)).expect("Port is already in use!")
        .run()
        .await;
}
```
And now we have a concurrent, single-threaded webserver. Note that in production we would want to have tokio spawn multiple threads(via `tokio::main`) that can work off multiple tasks concurrently.
But types in tasks have to implement Send (and sometimes even Sync) which can be more difficult because it may move between threads(Because in our analogy the now multiple waiters would have to pass information over). 

Enjoy your food!
![Unicorn shaped spaghetti](images/unicorn-shaped-spagetthi.png)

## Credits
- [My lovely wife](https://github.com/Segelente) for the great analogy and keeping me motivated :)
- [Aaron Turons Blog Post](https://aturon.github.io/blog/2016/08/11/futures/), where he describes much of what initially made up async Rust along with a history of it
- [Tokios Docs](https://tokio.rs/tokio/tutorial/async), where it is even covered how to make your own mini-executor!
