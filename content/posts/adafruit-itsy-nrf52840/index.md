+++
title =  "Getting Rust to run on the Adafruit nRF52840"
date = "2022-07-24T14:51:48+02:00"
author = ""
toc = false
draft = true
description = ""
keywords = ["rust", "embedded", "nRF52"]
tags = ["rust", "embedded", "nRF52", "microcontrollers"]
+++

Frustratingly it is not really easy to set Up Rust for the Adafruit itsybitsy nRF52840.

I originally read [this blog post](https://TODO)

Then discovered [this adafruit-nrf52-bluefruit-le Repo](https://github.com/nrf-rs/adafruit-nrf52-bluefruit-le)
which works better but the generated hex file in the second step only results in this:
```
0000000 303a 3030 3030 3030 4631 0d46 000a
000000d
```
which causes an error in the next step since adafruit-nrfutil tries to get the maxaddress of the blinky.hex file, 
which is in this case none since the hex file is apparently empty because there are no dict keys in there.
This is because self._buf is empty