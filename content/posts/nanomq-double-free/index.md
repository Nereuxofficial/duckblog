+++  
title = "How NanoMQ had a double free and what we can learn from it"  
date = "2023-04-25T14:31:55+02:00"  
author = ""  
authorTwitter = "" #do not include @  
cover = ""  
tags = ["C", "nanomq", "security", "fuzzing"]  
keywords = ["C", "nanomq", "security", "fuzzing"]  
description = "Why using Rust is a good idea"  
showFullContent = false  
draft = true  
+++  
People often ask me why I use Rust for my projects. I usually answer that I like the language and that it is a good fit  
for my use cases. But there is another reason: I came from C++, but I never really liked it. I always felt that it was  
overly complex and it was really easy to make grave mistakes. So in this post I want to show you how I found a double  
free in NanoMQ, an MQTT broker written in C, and what we can learn from it.

## Introduction to Undefined Behavior
Coming from a high-level language like Python or Java, you might not be familiar with the concept of undefined behavior.  
In C/C++ (and even in unsafe Rust!) you can write code that is not defined by the language. [This allows the compiler to  
make optimizations that would not be possible otherwise](https://blog.llvm.org/2011/05/what-every-c-programmer-should-know_14.html)  
and while most C/C++ programmers are aware of this, some of them don't know the full extent of what is and what is not  
undefined behavior in C/C++. For example, the following code is [undefined behavior in C/C++](https://blog.regehr.org/archives/213):
```c  
#include <limits.h>  
#include <stdio.h>  
  
int main (void)  
{  
printf ("%d\n", (INT_MAX+1) < 0);  
return 0;  
}  
```  
A simple overflow. And while when you run this code on your machine, it will probably print `1`, it is not guaranteed to  
do so, however the compiler is allowed to assume that the value is bigger than 0 and optimize the code  
accordingly(unless compiled with -fwrapv). And while this is just one example of UB in C/C++, there are many more.  
Now what happens when the compiler optimizes the code in a way that is not intended by the programmer? Well all garuantees the language gives you are off, so
[pretty much everything](https://blog.regehr.org/archives/213).

Here are some other examples of undefined behavior in C/C++:
#todo
Now there are many tools to detect all kinds of UB(the -ftrapv flag, valgrind)
And the larger your codebase gets the harder it gets to spot this UB and even projects like the Linux kernel with really
skilled developers and many eyes looking at the code have memory bugs in them.
## The Fuzzing
The project was very simple: Develop a simple MQTT Broker that is secure while also being fast. During the project
we used fuzzing to find bugs in our code and test the security of the broker. Initially the plan was to use Honggfuzz, 
which worked great with our Rust code and our MQTT decoding dependency, but then we had the idea to use it to fuzz other
fuzzers as well. And while it was really easy to fuzz a function which decodes MQTT packets, passed in as raw bytes, 
which is really easy to fuzz. Here is the code for the function:
```rust
extern crate core;
use bytes::BytesMut;
use honggfuzz::fuzz;
use mqtt_v5_fork::decoder::decode_mqtt;
use mqtt_v5_fork::types::ProtocolVersion;

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            if let Ok(packet) = decode_mqtt(&mut BytesMut::from(data), ProtocolVersion::V500) {
                let _ = packet;
            }
        });
    }
}
```
Meanwhile NanoMQ relied on a lot of global state and was not really easy to fuzz and when I finally had written
a fuzzing function for it it complained that it didn't run in a multithreaded environment.

%Coolduck says%
Why would you create a fuzzing harness for every single Broker(especially with your lacking C knowledge)? 
Couldn't you just fuzz a running MQTT broker?
%coolduck%

That's a great idea!

So I searched for other fuzzers and found [FUME](https://github.com/PBearson/FUME-Fuzzing-MQTT-Brokers).
FUME uses Markov Modeling to generate MQTT packets and then sends them to the broker.
So it takes away the work of creating fuzzing harnesses for every broker and (at the cost of efficiency) allows you to 
fuzz any MQTT broker(and it was also used to find a security vulnerability in Mosquitto!).
So I started FUME and let it run with the following brokers:
- MCloudTT(Rust)(our broker)
- mosquitto(C)
- NanoMQ(C)
- EMQX(Erlang)
- HiveMQ(Java)

And after waiting a while...
<TODO: Insert Image>
## The Bug
