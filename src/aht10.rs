use embedded_hal::{delay::DelayNs, i2c::I2c};

pub const AHT10_ADDR: u8 = 0x38;

const CMD_SOFT_RESET: u8 = 0xBA;
const CMD_INIT: [u8; 3] = [0xE1, 0x08, 0x00];
const CMD_MEASURE: [u8; 3] = [0xAC, 0x33, 0x00];
const STATUS_BUSY_BIT: u8 = 0x80;
const STATUS_CAL_BIT: u8 = 0x08;

const RESOLUTION_20_BIT: f32 = 1048576.0;

#[derive(Debug)]
pub enum Aht10Error<E> {
    I2c(E),
    NotCalibrated,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub temperature_c: f32,
    pub hum_percent: f32,
}

pub struct Aht10<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C, E> Aht10<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: AHT10_ADDR,
        }
    }

    pub fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Aht10Error<E>> {
        delay.delay_ms(20); // Delay incase the sensor doesn't go idle state after vdd

        self.i2c
            .write(self.addr, &[CMD_SOFT_RESET])
            .map_err(Aht10Error::I2c)?;
        delay.delay_ms(20);

        let status: u8 = self.read_status()?;
        if status & STATUS_CAL_BIT == 0 {
            self.i2c
                .write(self.addr, &CMD_INIT)
                .map_err(Aht10Error::I2c)?;
            delay.delay_ms(10);
        }

        let status: u8 = self.read_status()?;
        if status & STATUS_CAL_BIT == 0 {
            return Err(Aht10Error::NotCalibrated);
        }

        Ok(())
    }

    fn read_status(&mut self) -> Result<u8, Aht10Error<E>> {
        let mut buf: [u8; 1] = [0u8; 1];
        self.i2c
            .read(self.addr, &mut buf)
            .map_err(Aht10Error::I2c)?;
        Ok(buf[0])
    }

    pub fn read<D: DelayNs>(&mut self, delay: &mut D) -> Result<Measurement, Aht10Error<E>> {
        self.i2c
            .write(self.addr, &CMD_MEASURE)
            .map_err(Aht10Error::I2c)?;
        delay.delay_ms(80);

        let mut buf: [u8; 6] = [0u8; 6];
        let mut been_read: bool = false;
        for _ in 0..10 {
            self.i2c
                .read(self.addr, &mut buf)
                .map_err(Aht10Error::I2c)?;

            if buf[0] & STATUS_BUSY_BIT == 0 {
                been_read = true;
                break;
            }

            delay.delay_ms(10);
        }

        if !been_read {
            return Err(Aht10Error::Timeout);
        }

        let raw_hum: u32 =
            ((buf[1] as u32) << 12) | ((buf[2] as u32) << 4) | ((buf[3] as u32) >> 4);
        let raw_temp: u32 =
            (((buf[3] as u32) & 0xf) << 16) | ((buf[4] as u32) << 8) | (buf[5] as u32);

        let humidty: f32 = (raw_hum as f32 / RESOLUTION_20_BIT) * 100.0;
        let temperature: f32 = ((raw_temp as f32 / RESOLUTION_20_BIT) * 200.0) - 50.0;

        Ok(Measurement {
            hum_percent: humidty,
            temperature_c: temperature,
        })
    }
}
