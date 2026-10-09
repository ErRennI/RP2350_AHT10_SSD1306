use core::convert::Infallible;

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::OriginDimensions,
    pixelcolor::BinaryColor,
    prelude::{Pixel, Size},
};
use embedded_hal::i2c::I2c;

pub const SSD1306_ADDR: u8 = 0x3C;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const PAGES: usize = HEIGHT / 8;

const INIT_SEQUENCE: [u8; 25] = [
    0xAE, // 1. Display OFF
    0xD5, 0x80, // 2. Set Clock Divide Ratio / Osc Frequency (Ratio: 1, Freq: 8)
    0xA8, 0x3F, // 3. Set Multiplex Ratio (64 MUX = 0x3F)
    0xD3, 0x00, // 4. Set Display Offset (0 = no offset)
    0x40, // 5. Set Display Start Line (Line 0)
    0x8D, 0x14, // 6. Enable Charge Pump Regulator (CRITICAL)
    0x20, 0x00, // 7. Set Memory Addressing Mode to Horizontal (0x00)
    0xA1, // 8. Set Segment Re-map: col 127 -> SEG0 (Horizontal Flip for standard mounting)
    0xC8, // 9. Set COM Output Scan Direction: COM[N-1] to COM0 (Vertical Flip)
    0xDA, 0x12, // 10. Set COM Pins Hardware Configuration (Alternative layout for 128x64)
    0x81, 0xCF, // 11. Set Contrast Control (0xCF = 207)
    0xD9, 0xF1, // 12. Set Pre-charge Period (Phase 1: 1 DCLK, Phase 2: 15 DCLK)
    0xDB, 0x40, // 13. Set VCOMH Deselect Level (~0.83 * Vcc)
    0xA4, // 14. Entire Display ON from RAM
    0xA6, // 15. Set Normal Display (1 = Pixel Lit)
    0xAF, // 16. Display ON
];

#[derive(Debug)]
pub enum Ssd1306Error<E> {
    I2c(E),
}

pub struct Ssd1306<I2C> {
    i2c: I2C,
    addr: u8,
    buf: [u8; WIDTH * PAGES],
}

impl<I2C, E> Ssd1306<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: SSD1306_ADDR,
            buf: [0; WIDTH * PAGES],
        }
    }

    fn cmd(&mut self, cmds: &[u8]) -> Result<(), Ssd1306Error<E>> {
        for &c in cmds {
            self.i2c
                .write(SSD1306_ADDR, &[0x00, c])
                .map_err(Ssd1306Error::I2c)?;
        }
        Ok(())
    }

    pub fn init(&mut self) -> Result<(), Ssd1306Error<E>> {
        self.cmd(&INIT_SEQUENCE)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Ssd1306Error<E>> {
        self.cmd(&[0x21, 0, 127, 0x22, 0, 7])?;
        let mut pckt: [u8; 129] = [0u8; 129];
        pckt[0] = 0x40;

        for chunk in self.buf.chunks(128) {
            pckt[1..(1 + chunk.len())].copy_from_slice(chunk);

            self.i2c
                .write(SSD1306_ADDR, &pckt)
                .map_err(Ssd1306Error::I2c)?;
        }
        Ok(())
    }
}

impl<I2C> OriginDimensions for Ssd1306<I2C> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<I2C> DrawTarget for Ssd1306<I2C> {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.x >= WIDTH as i32 || point.y < 0 || point.y >= HEIGHT as i32 {
                continue;
            }
        }

        todo!()
    }
}
