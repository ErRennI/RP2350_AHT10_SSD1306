use core::convert::Infallible;

// If you are using embeded_graphics_core add "_core" to the end of embeded_graphics
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions},
    pixelcolor::BinaryColor,
    prelude::{Pixel, Size},
    primitives::Rectangle,
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
        let mut pckt: [u8; 33] = [0u8; 33];
        for chunk in cmds.chunks(32) {
            pckt[1..=chunk.len()].copy_from_slice(chunk);
            self.i2c
                .write(self.addr, &pckt[..=chunk.len()])
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
                .write(self.addr, &pckt)
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

            let index: usize = point.x as usize + (point.y as usize / 8) * WIDTH;
            let bit: u8 = 1 << (point.y % 8) as u8;
            match color {
                BinaryColor::On => self.buf[index] |= bit,
                BinaryColor::Off => self.buf[index] &= !bit,
            };
        }

        Ok(())
    }
    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        match color {
            BinaryColor::On => self.buf.fill(0xFF),
            BinaryColor::Off => self.buf.fill(0x00),
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let drawable_area: Rectangle = area.intersection(&self.bounding_box());

        if drawable_area.is_zero_sized() {
            return Ok(());
        }

        let x_start: usize = drawable_area.top_left.x as usize;
        let x_end: usize = x_start + drawable_area.size.width as usize;
        let y_start: usize = drawable_area.top_left.y as usize;
        let y_end: usize = y_start + drawable_area.size.height as usize;

        for page in (y_start / 8)..=((y_end - 1) / 8) {
            let page_top: usize = page * 8;
            let first: usize = y_start.max(page_top) - page_top;
            let last: usize = y_end.min(page_top + 8) - page_top;
            let mask: u8 = (0xFF << first) & (0xFF >> (8 - last));

            for x in x_start..x_end {
                let index: usize = x + page * WIDTH;

                match color {
                    BinaryColor::On => self.buf[index] |= mask,
                    BinaryColor::Off => self.buf[index] &= !mask,
                }
            }
        }
        Ok(())
    }
}
