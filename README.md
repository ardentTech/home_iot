# Home IOT

Firmware for a remote device that reads sensor data and then transmits payloads via LoRa.

### Hardware
- [RP Pico 2W](https://www.adafruit.com/product/6087)
- [Semtech sx127x](https://www.adafruit.com/product/3072)
- [Honeywell MPR](https://www.adafruit.com/product/3965)
- [NXP PCF8523T](https://www.nxp.com/part/PCF8523T)
- [Pico Debug Probe](https://www.adafruit.com/product/5699) (optional but helpful)

### Software

This [embassy](https://github.com/embassy-rs/embassy) project uses the ARM core(s) of the RP2350 and leverages three device drivers that I authored and maintain:

* [sx127x-lora](https://github.com/ardentTech/sx127x-lora)
* [honeywell-mpr](https://github.com/ardentTech/honeywell-mpr)
* [nxp-pcf8523](https://github.com/ardentTech/nxp-pcf8523)

![Architecture](/assets/software_arch.png)

### TODO

- [ ] add sdcard for logging
- [ ] expand command response system
- [x] system diagram(s)
- [x] pressure sensor
- [x] air quality sensor
- [x] command and response over UART
- [ ] dormant + wake