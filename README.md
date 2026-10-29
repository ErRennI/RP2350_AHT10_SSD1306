# RP2350 Temperature & Humidity Monitor

Bare-metal Rust firmware for the RP2350 that reads temperature and humidity from an AHT10 sensor and shows the readings on an SSD1306 OLED display. Both device drivers are written from scratch against the `embedded-hal` I2C trait.

<img width="600" alt="RP2350 with AHT10 sensor and SSD1306 display showing temperature and humidity" src="https://github.com/user-attachments/assets/40fdac1a-bc5a-4c6b-8893-1ef074d68255" />

## Features

- `no_std` / `no_main` firmware on `rp235x-hal`, with no OS or RTOS
- **AHT10 driver** (`src/aht10.rs`): soft reset, calibration check and init, measurement trigger, busy-bit polling with timeout, and 20-bit raw-to-physical conversion
- **SSD1306 driver** (`src/ssd1306.rs`): 128x64 init sequence, 1 KB framebuffer, page-addressed flush over I2C, and an `embedded-graphics` `DrawTarget` implementation (including an optimized `fill_solid`)
- One shared 400 kHz I2C bus for both devices via `embedded-hal-bus`
- Reliability handling:
  - 2 second hardware watchdog (a panic or hang ends in a reset)
  - Retried initialization for both devices
  - Chip reset after 5 consecutive I2C errors

## Hardware

- An RP2350-based board
- AHT10 temperature and humidity sensor (I2C address `0x38`)
- SSD1306 128x64 OLED display (I2C address `0x3C`)

### Wiring

Both devices share the same I2C bus (I2C0):

| Signal | RP2350 pin |
| ------ | ---------- |
| SDA    | GPIO4      |
| SCL    | GPIO5      |
| VCC    | 3V3        |
| GND    | GND        |

Internal pull-ups are enabled on SDA and SCL in firmware.

## What it shows

The display refreshes about every 500 ms with two lines of text:

```
Temp: 23.4 C
Hum: 45.6 %
```

## Project structure

```
src/
├── main.rs    # Clocks, watchdog, shared I2C bus, main loop, error handling
├── aht10.rs   # AHT10 sensor driver
└── ssd1306.rs # SSD1306 display driver + embedded-graphics DrawTarget
```

## Building

Requires a stable Rust toolchain with the target configured in `.cargo/config.toml`.

```
cargo build --release
```

Flash the resulting binary (`HUM_TEMP`) to the board with your usual tool, for example `picotool` or a UF2 workflow.

## Dependencies

- [`rp235x-hal`](https://crates.io/crates/rp235x-hal), `cortex-m`, `cortex-m-rt`
- [`embedded-hal`](https://crates.io/crates/embedded-hal) 1.0, `embedded-hal-bus`
- [`embedded-graphics`](https://crates.io/crates/embedded-graphics)
- [`heapless`](https://crates.io/crates/heapless) for allocation-free string formatting

## Known limitations

- Text-only output with a fixed 128x64 display and fixed I2C addresses
- No sleep or low-power mode

