# Home IOT

Rust `#![no_std]` firmware for a remote device that reads from various sensors and then transmits payloads via LoRa.

### Hardware
- [RP Pico 2W](https://www.adafruit.com/product/6087)
- [Semtech sx127x](https://www.adafruit.com/product/3072)
- [Honeywell MPR](https://www.adafruit.com/product/3965)
- [NXP PCF8523T](https://www.nxp.com/part/PCF8523T)
- [Pico Debug Probe](https://www.adafruit.com/product/5699) (optional but helpful)

### Software

The [Embassy](https://github.com/embassy-rs/embassy) app uses the ARM core(s) of the RP2350 and leverages three device
drivers that I authored and maintain:

* [sx127x-lora](https://github.com/ardentTech/sx127x-lora)
* [honeywell-mpr](https://github.com/ardentTech/honeywell-mpr)
* [nxp-pcf8523](https://github.com/ardentTech/nxp-pcf8523)

![Architecture](/assets/software_arch.png)

#### Command + Response

UART-based command + response component allows the host to interact with the target system by issuing pre-defined
commands:

| Command | Description                           |
|---------|---------------------------------------|
| add     | add one second to RTC datetime        |
| led     | turn the LED on for one second        |
| now     | read the current RTC timestamp        |
| sub     | subtract one second from RTC datetime |

The commands are (currently) case-sensitive, and are actively being developed.

### TODO

- [ ] add sdcard for logging
- [x] system diagram(s)
- [x] pressure sensor
- [x] air quality sensor
- [x] command and response over UART
- [ ] dormant + wake (need secure boot)
- [ ] detect UART connection and set flag for cmd bus
- [x] add commands for yellow and red LEDs
- [ ] green, yellow and red LEDs for AQI? app state?
- [ ] UART help command
- [ ] display mode as LED flag?