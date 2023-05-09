+++
title = "Making a Dino Light with the ESP32 and WS2812 Pt. 2"
date = "2023-04-22T21:31:55+02:00"
author = ""
authorTwitter = "" #do not include @
cover = ""
tags = ["rust", "embedded", "esp32", "ws2812", "neopixel"]
keywords = ["rust", "embedded", "esp32", "ws2812", "neopixel"]
description = "Connecting an ESP32 Dino Light to Wi-Fi using Rust and Embassy"
showFullContent = false
+++
If you haven't read the first part, you can do so [here](/posts/esp32-ws2812-dino-light/).

Hey, it's been a while. There has been a lot that happened since the first part(which was last year), and the esp-rs 
ecosystem has improved quite a lot. With no-std wifi support and support for async among many things. Now I hear you
say "async? On an embedded device?". Well, yes. And it's actually quite nice.

## Introducing Embassy
[Embassy](https://embassy.dev/) is an async runtime for bare-metal Rust, removing the need for a RTOS like 
FreeRTOS(or TockOS, [which recently also got support for the esp32c3](https://www.tockos.org/)). It's still in early 
development, but it's already quite usable. But since this is bleeding-edge stuff there are some rough edges.
Essentially it's a bit like tokio or async-std but for bare-metal. You have an executor, which runs your tasks and if 
there is an await in your code it will suspend the task and poll all tasks until one of them is ready to continue. 
Sadly those tasks don't support generics as of now, but I found a workaround for that in this project.


## Programming the ESP32
It took me quite a while to figure this out, but the people in the esp-rs matrix channel were very helpful.
Here are the many but necessary dependencies along with their features:
```toml
[dependencies]
# Much, much esp-stuff, this time with async support
hal = { package = "esp32-hal", version="0.11.0", features = [
    "embassy",
    "async",
    "rt",
    "embassy-time-timg0",
] }
esp-backtrace = { version = "0.6.0", features = [
    "esp32",
    "panic-handler",
    "exception-handler",
    "print-uart",
] }
esp-println = { version = "0.4.0", features = ["esp32", "log"] }
esp-alloc = { version = "0.2.0", features = ["oom-handler"] }
esp-wifi = { git = "https://github.com/esp-rs/esp-wifi", features = [
    "esp32",
    "esp32-async",
    "async",
    "embedded-svc",
    "embassy-net",
    "wifi",
] }
embedded-svc = { version = "0.23.1", default-features = false, features = [] }
embedded-io = "0.4.0"
# Embassy, our async runtime
embassy-sync = "0.1.0"
embassy-time = { version = "0.1.0", features = ["nightly"] }
embassy-executor = { package = "embassy-executor", git = "https://github.com/embassy-rs/embassy/", rev = "cd9a65b", features = [
    "nightly",
    "integrated-timers",
] }
embassy-net-driver = { git = "https://github.com/embassy-rs/embassy", rev = "26474ce6eb759e5add1c137f3417845e0797df3a" }
embassy-net = { git = "https://github.com/embassy-rs/embassy", rev = "26474ce6eb759e5add1c137f3417845e0797df3a", features = [
    "nightly",
    "tcp",
    "udp",
    "dhcpv4",
    "medium-ethernet",
] }
futures-util = { version = "0.3.17", default-features = false }

# Serde_json without needing allocations
serde = { version = "1.0", default-features = false }
serde-json-core = "0.5.0"
# LED strip driver, this time without even needing SPI
smart-leds = "0.3.0"
esp-hal-smartled = {version = "0.1.0", features = ["esp32"]}
# For global channel(Workaround for lifetime restrictions on tasks) 
lazy_static = { version = "1.4.0", features = ["spin_no_std"] }

# This is necessary in order for WIFI to work
[profile.dev.package.esp-wifi]
opt-level = 3
[profile.release]
opt-level = 3
lto="off"
```
Install the latest version of [espflash](https://github.com/esp-rs/espflash) via: `cargo install espflash --git https://github.com/esp-rs/espflash`.
And we need the following in our `.cargo/config.toml`:
```toml
[target.xtensa-esp32-none-elf]
runner = "espflash flash --monitor"

[build]
rustflags = [
  "-C", "link-arg=-Tlinkall.x",
  # In order for esp-wifi to work we need this linker argument  
  "-C", "link-arg=-Trom_functions.x",
  "-C", "link-arg=-nostartfiles",
]
# ESP32 fist-gen
target = "xtensa-esp32-none-elf"

[unstable]
# Strictly speaking alloc was not necessary for this project but it's really useful if you ever want to use a heap
build-std = ["alloc", "core"]
```
And finally to avoid typing `cargo +esp` every time you use cargo use:
```
echo "[toolchain]
channel = \"esp\"" > rust-toolchain.toml
```
And finally we need to add the following to our `main.rs` to allocate our heap:
```rust
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
extern crate alloc;

use alloc::vec;
use esp_backtrace as _;
use esp_println::println;
use hal::entry;
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 2 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }
    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}
#[entry]
fn main() -> ! {
    init_heap();
    // And we can use the heap!
    println!("Vec element 0: {}", vec![1, 2, 3][0]);
    loop {}
}
```
And when we execute it we see this:
```
Vec element 0: 1
```
So we can use heap on this tiny microcontroller! And shortly after this it gets reset with this output:
```
ets Jun  8 2016 00:22:57

rst:0x10 (RTCWDT_RTC_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)
configsip: 0, SPIWP:0xee
clk_drv:0x00,q_drv:0x00,d_drv:0x00,cs0_drv:0x00,hd_drv:0x00,wp_drv:0x00
mode:DIO, clock div:2
load:0x3fff0030,len:7024
0x3fff0030 - _stack_end_cpu0
    at ??:??
load:0x40078000,len:15400
0x40078000 - ets_delay_us
    at ??:??
load:0x40080400,len:3816
0x40080400 - _init_end
    at ??:??
entry 0x40080648
0x40080648 - _ZN14esp_hal_common9interrupt6xtensa8vectored17handle_interrupts17hb0c80caf10c2f321E
    at ??:??
## The rest normal boot sequence follows after this
```
Which is weird because we didn't even use any interrupts...

If only there was somebody to help me with this...

%Coolduck says%
Almost like your code got reset because it wasn't doing anything useful.
%coolduck%

I guess you're right.

%Coolduck says%
In embedded devices there often is a Watchdog timer that resets the device if it doesn't get fed to avoid deadlocks.
%coolduck%

Oh I see, so I need to feed it somehow.

%Coolduck says%
Yes that would be the proper way, or you disable it. Your choice.
%coolduck%

What a cool duck.
I'll disable it for now(Not only because I'm lazy but also because esp-wifi does it too in their example).
So upon reading into the [example](https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs),
we find that this disables the watchdog:

```rust
#[entry]
fn main() -> ! {
    // All peripherals of our chip
    let peripherals = Peripherals::take();
    // Take the Systemparts, containing the clock control, cpo_control and even radio_clock_control, which we're gonna get to later
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze();
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    rtc.rwdt.disable();
}
```

Now onto getting WI-FI working. Shouldn't be too hard right?
```rust
// Straight up copied from the dhcp example
const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}
fn main() -> ! {
    init_heap();

    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze();
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    rtc.rwdt.disable();

    let timer = TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    initialize(
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
        .unwrap();
    // Now we get to use the radio peripherals!
    let (wifi, _) = peripherals.RADIO.split();
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(wifi, WifiMode::Sta);

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0.timer0);

    let config = Config::Dhcp(Default::default());

    let seed = 123456; // Change this for your own project

    // Init network stack
    let stack = &*singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(StackResources::<3>::new()),
        seed
    ));

    // Initialize the embassy executor
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(connection(controller)).ok();
        spawner.spawn(net_task(stack)).ok();
    });
}
/// I really don't know why we need this but it's necessary
#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static>>) {
    stack.run().await
}
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                println!("Wifi disconnected!");
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}
```
Now when we export the SSID and PASSWORD via Console we can connect to wifi!
```bash
export SSID="your SSID" PASSWORD="your password"
cargo run -q --release
```
Now we can connect and use our LED strip. If you remember the last part, we used SPI(Serial Peripheral Interface) for this,
but it was quite tedious to use. Luckily, the Espressif Rust team has created esp-hal-smartled which allows us to use an 
RMT output channel, while still having the convenience functions of the smart-leds crate.
```rust
// in our main
let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
let pulse = PulseControl::new(peripherals.RMT, &mut system.peripheral_clock_control).unwrap();
let mut led = <smartLedAdapter!(23)>::new(pulse.channel0, io.pins.gpio33);
// Turn the lights off by default
led.write([RGB8::default(); 23].into_iter()).unwrap();
```
And we can make a seperate task for this, always waiting for a channel to have a new Message:
```rust
// Since lifetime limitations don't allow us to pass a receiver and sender to the webserver and
// led_task threads, this is an acceptable workaround. This channel is over a
// CriticalsectionRawMutex, since lazy_static requires the Mutex to be Thread-safe. In there we
// store up 3 RGB8 values and when full, sending will (a)wait until a message is received.
lazy_static! {
    static ref CHANNEL: Channel<CriticalSectionRawMutex, OwnRGB8, 3> =
        embassy_sync::channel::Channel::new();
}
#[embassy_executor::task]
// This is a really long type, but since embassy doesn't support generics yet, we have to specify the type fully
async fn led_task(
    mut leds: SmartLedsAdapter<
        ConfiguredChannel0<
            'static,
            GpioPin<
                hal::gpio::Unknown,
                Bank1GpioRegisterAccess,
                DualCoreInteruptStatusRegisterAccessBank1,
                InputOutputAnalogPinType,
                Gpio33Signals,
                33,
            >,
        >,
        553,
    >,
) {
    loop {
        println!("Waiting for color...");
        let receiver = CHANNEL.receiver();
        let color: RGB8 = receiver.recv().await.into();
        // If you send an array of colours you could also color the LEDs differently but in what is typical Go fashion
        // that is left as an exercise to the reader(See https://fasterthanli.me/articles/lies-we-tell-ourselves-to-keep-using-golang#go-as-a-prototyping-starter-language)
        leds.write([color; 23].into_iter()).unwrap();
    }
}
```
Lastly, we have the server code.
Building an HTTP Server on bare-metal hardware is fairly easy thanks to the abstractions embassy and esp-hal provide.
```rust
#[embassy_executor::task]
async fn task(stack: &'static Stack<WifiDevice<'static>>) {
    // Wait until Wifi is connected
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    // Wait for DHCP to get an IP address
    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    println!("Starting web server...");
    // Get our sender
    let sender = CHANNEL.sender();
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_net::SmolDuration::from_secs(10)));
    loop {
        println!("Wait for connection...");
        // We can easily bind our socket to port 80, and accept connections
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;
        println!("Connected...");

        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }

        use embedded_io::asynch::Write;

        let mut buffer = [0u8; 512];
        let mut pos = 0;
        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(len) => {
                    let to_print =
                        unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

                    println!("read {} bytes: {}", len, to_print);
                    if to_print.starts_with("POST") {
                        let r = socket.write_all(b"HTTP/1.0 200 OK\r\n\r\n").await;
                        if let Err(e) = r {
                            println!("write error: {:?}", e);
                        }
                        let r = socket.flush().await;
                        if let Err(e) = r {
                            println!("flush error: {:?}", e);
                        }
                        // I couldn't find a library for no_std http parsing but this works as well and is fairly simple
                        if let Some(body) =
                            to_print.lines().into_iter().find(|l| l.starts_with("{"))
                        {
                            println!("Body: {}", body);
                            if let Ok((color, _)) = serde_json_core::from_str::<OwnRGB8>(body) {
                                println!("Got color: {:?}", color);
                                sender.send(color).await;
                            }
                        }
                    }

                    pos += len;
                }
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
            Timer::after(Duration::from_millis(100)).await;
            socket.close();
            Timer::after(Duration::from_millis(100)).await;
            socket.abort();
        }
    }
}
```
And now we can control it via this curl POST request:
```bash
curl -v -d '{"r":0,"g":0,"b":200}' http://<ESP-IP-ADDRESS>
```

%Coolduck says%
And the LEDs turn blue, how nice! I'm sure you can figure out how to make it turn red and green as well.
%coolduck%

There is a lot more we can do with this, for example, do rainbow colors with embassy Timers, accept GET requests to show
a simple web client(implemented in the repository) or enable the user to light up the LEDs in a certain pattern.

That's it for this post. I hope you enjoyed it and maybe even learned something. I'd like to make my future posts more 
indepth and technical, but I'm not sure if I can do that without making them too long. If you have any suggestions,
feel free to open an issue on the repository or contact me on [Mastodon](https://infosec.exchange/@Nereuxofficial).

## The Code
As always the repository is freely available here:
[https://github.com/Nereuxofficial/nostd-wifi-lamp](https://github.com/Nereuxofficial/nostd-wifi-lamp)

The Repository also has support for some exciting things like Wokwi, which provides a simulated ESP32-C3 thanks to being created 
with the [esp-template](https://github.com/esp-rs/esp-template).
## Thanks to:
[bjoernQ](https://github.com/bjoernQ) for fixing an error where the stack overflowed into the heap,
and I had no idea why it was crashing.

[esp-rs](https://github.com/esp-rs) for the awesome tooling around ESP32 Microcontrollers.