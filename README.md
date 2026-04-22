# Home IOT

Firmware for a remote device that reads sensor data and then transmits payloads via LoRa.

### Hardware
- [RP Pico 2W](https://www.adafruit.com/product/6087)
- [Semtech sx127x](https://www.adafruit.com/product/3072)
- [Honeywell MPR](https://www.adafruit.com/product/3965)
- [Adalogger Featherwing](https://www.adafruit.com/product/2922)
- [Pico Debug Probe](https://www.adafruit.com/product/5699)

### Software

This [embassy](https://github.com/embassy-rs/embassy) project uses the ARM core and leverages three device drivers that I authored and maintain:

* [sx127x-lora](https://github.com/ardentTech/sx127x-lora)
* [honeywell-mpr](https://github.com/ardentTech/honeywell-mpr)
* [nxp-pcf8523](https://github.com/ardentTech/nxp-pcf8523)

![Organigram](/assets/organigram.png)

### TODO

- [ ] initialize nxp-pcf8523 datetime at compile time?
- [ ] disable rp235x rtc
- [ ] add sdcard
- [x] system diagram(s)
- [x] more sensors
- [ ] debug command and response over UART