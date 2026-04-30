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

| Command           | Args | Description                           |
|-------------------|------|---------------------------------------|
| green_led_toggle  |      | toggle the green LED                  |
| red_led_toggle    |      | toggle the red LED                    |
| rtc_add_sec       |      | add one second from RTC datetime      |
| rtc_now           |      | print the current timestamp           |
| rtc_set_day       | <u8> | set the RTC day                       |
| rtc_set_hour      | <u8> | set the RTC hour                      |
| rtc_set_min       | <u8> | set the RTC minute                    |
| rtc_set_month     | <u8> | set the RTC month                     |
| rtc_set_sec       | <u8> | set the RTC second                    |
| rtc_set_year      | <u8> | set the RTC year                      |
| rtc_sub_sec       |      | subtract one second from RTC datetime |
| yellow_led_toggle |      | toggle the yellow LED                 |

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