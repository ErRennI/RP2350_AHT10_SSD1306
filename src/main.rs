#![no_std]
#![no_main]

mod aht10;
mod ssd1306;

// Write my own panic handler
use core::panic::PanicInfo;

// Share i2c bus
use core::cell::RefCell;
use embedded_hal_bus::i2c::RefCellDevice;

use embedded_hal::{
    delay::{self, DelayNs},
    i2c::I2c,
};

//Alias
use rp235x_hal::{
    self as hal, Sio, Watchdog,
    clocks::ClocksManager,
    fugit::RateExtU32,
    gpio::{
        FunctionI2C, Pin, Pins, PullUp,
        bank0::{Gpio4, Gpio5},
    },
    pac::Peripherals,
};

use crate::aht10::{Aht10, Aht10Error, Measurement};

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

    let mut i2c_error_count: u8 = 0;
    loop {
        let measurement: Measurement = match aht10.read(&mut timer) {
            Ok(measurement) => measurement,
            Err(Aht10Error::I2c(_)) => {
                i2c_error_count += 1;
                if i2c_error_count >= 5 {
                    cortex_m::peripheral::SCB::sys_reset();
                }
                continue;
            }
            Err(_) => {
                continue;
            }
        };

        timer.delay_ms(1000);
    }
}
