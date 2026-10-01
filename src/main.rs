#![no_std]
#![no_main]

mod aht10;

// Write my own panic handler
use core::panic::PanicInfo;

use embedded_hal::i2c::I2c;

//Alias
use rp235x_hal::{
    self as hal, Watchdog,
    clocks::ClocksManager,
    fugit::RateExtU32,
    pac::{I2C1, Peripherals},
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
    let mut pac: Peripherals = hal::pac::Peripherals::take().unwrap();
    // Fast gpio
    let sio: Sio = hal::Sio::new(pac.SIO);

    let mut watchdog: Watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks: ClocksManager = hal::clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins: Pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_sensor: hal::gpio::Pin<Gpio4, hal::gpio::FunctionI2C, hal::gpio::PullUp> =
        pins.gpio4.reconfigure();

    let scl_sensor: hal::gpio::Pin<Gpio5, hall::gpio::FunctionI2C, hal::gpio::PullUp> =
        pins.gpio5.reconfigure();

    let mut i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_sensor,
        scl_sensor,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    loop {}
}
