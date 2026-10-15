#![no_std]
#![no_main]

mod aht10;
mod ssd1306;

// Write my own panic handler
use core::fmt::Write;
use core::panic::PanicInfo;

// Share i2c bus
use core::cell::RefCell;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    text::Text,
};
use embedded_hal_bus::i2c::RefCellDevice;

use embedded_hal::delay::DelayNs;

//Alias
use rp235x_hal as hal;
use rp235x_hal::{
    Sio, Watchdog,
    clocks::ClocksManager,
    fugit::{ExtU32, RateExtU32},
    gpio::{
        FunctionI2C, Pin, Pins, PullUp,
        bank0::{Gpio4, Gpio5},
    },
    pac::Peripherals,
};

use crate::ssd1306::Ssd1306Error;
use crate::{
    aht10::{Aht10, Aht10Error, Measurement},
    ssd1306::Ssd1306,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Copy metadata to .start_block so boot ROM knows how to boot our program
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

// Main entry point
#[hal::entry]
fn main() -> ! {
    let mut pac: Peripherals = match hal::pac::Peripherals::take() {
        Some(c) => c,
        None => {
            cortex_m::peripheral::SCB::sys_reset();
        }
    };
    // Fast gpio
    let sio: Sio = Sio::new(pac.SIO);

    let mut watchdog: Watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks: ClocksManager = match hal::clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ) {
        Ok(c) => c,
        Err(_) => {
            cortex_m::peripheral::SCB::sys_reset();
        }
    };

    watchdog.start(2000.millis());

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let pins: Pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_sensor: Pin<Gpio4, FunctionI2C, PullUp> = pins.gpio4.reconfigure();

    let scl_sensor: Pin<Gpio5, FunctionI2C, PullUp> = pins.gpio5.reconfigure();

    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_sensor,
        scl_sensor,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let i2c_bus = RefCell::new(i2c);

    let mut aht10 = Aht10::new(RefCellDevice::new(&i2c_bus));
    match aht10.init(&mut timer) {
        Ok(_) => (),
        Err(Aht10Error::NotCalibrated) => {
            timer.delay_ms(50);

            if aht10.init(&mut timer).is_err() {
                cortex_m::peripheral::SCB::sys_reset();
            }
        }
        Err(_) => {
            cortex_m::peripheral::SCB::sys_reset();
        }
    }

    let mut ssd1306 = Ssd1306::new(RefCellDevice::new(&i2c_bus));
    match ssd1306.init() {
        Ok(_) => (),
        Err(_) => {
            timer.delay_ms(50);
            if ssd1306.init().is_err() {
                cortex_m::peripheral::SCB::sys_reset();
            }
        }
    };

    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let mut text: heapless::String<20> = heapless::String::new();

    let mut i2c_error_count: u8 = 0;

    loop {
        watchdog.feed();
        let measurement: Measurement = match aht10.read(&mut timer) {
            Ok(measurement) => measurement,
            Err(Aht10Error::I2c(_)) => {
                i2c_error_count += 1;
                if i2c_error_count >= 5 {
                    cortex_m::peripheral::SCB::sys_reset();
                }
                timer.delay_ms(50);
                continue;
            }
            Err(_) => {
                timer.delay_ms(50);
                continue;
            }
        };

        let t: i32 = (measurement.temperature_c * 10.0) as i32;
        let h: i32 = (measurement.hum_percent * 10.0) as i32;
        ssd1306.clear(BinaryColor::Off).ok();

        text.clear();
        write!(text, "Temp: {}.{} C", t / 10, (t % 10).abs()).ok();
        Text::new(&text, Point::new(10, 10), style)
            .draw(&mut ssd1306)
            .ok();

        text.clear();
        write!(text, "Hum: {}.{} %", h / 10, h % 10).ok();
        Text::new(&text, Point::new(10, 24), style)
            .draw(&mut ssd1306)
            .ok();

        match ssd1306.flush() {
            Ok(()) => (),
            Err(Ssd1306Error::I2c(_)) => {
                i2c_error_count += 1;
                if i2c_error_count >= 5 {
                    cortex_m::peripheral::SCB::sys_reset();
                }
                timer.delay_ms(50);
                continue;
            }
        };

        i2c_error_count = 0;
        timer.delay_ms(500);
    }
}
