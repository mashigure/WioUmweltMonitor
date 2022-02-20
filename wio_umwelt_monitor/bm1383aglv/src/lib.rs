//! for Atmospheric Pressure Sensor

#![no_std]

use panic_halt as _;
use wio_terminal as wio;

use wio::hal::gpio::*;
use wio::hal::sercom::*;
use wio::prelude::*;
use wio::hal::delay::Delay;


pub struct BM1383AGLV {
    enable: bool,
}

pub enum ErrorBM1383AGLV {
    ReadFailure,
    WriteFailure,
    CanNotAccess,
    CanNotFind,
    CanNotWritePowDwn,
    CanNotWriteReset,
    CanNotWriteModeCtr,
    NotInitialized,
    NoData
}

impl BM1383AGLV {
    pub fn new() -> BM1383AGLV {
        BM1383AGLV {
            enable: false
        }
    }

    pub fn init(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, delay: &mut Delay) -> Result<(), ErrorBM1383AGLV> {

        match self.read_single(i2c, 0x10) {
            Ok(reg) => {
                if reg != 0x32 {
                    return Err(ErrorBM1383AGLV::CanNotFind);
                }
            },
            Err(_) => return Err(ErrorBM1383AGLV::CanNotAccess)
        }

        if let Err(_) = self.write_single(i2c, 0x12, 1) {
            return Err(ErrorBM1383AGLV::CanNotWritePowDwn);
        }

        // wait between power down and reset
        delay.delay_ms(2u16);

        if let Err(_) = self.write_single(i2c, 0x13, 1) {
            return Err(ErrorBM1383AGLV::CanNotWriteReset);
        }

        if let Err(_) = self.write_single(i2c, 0x14, 0xCA) {
            return Err(ErrorBM1383AGLV::CanNotWriteModeCtr);
        }

        delay.delay_ms(240u16);

        self.enable = true;
        Ok(())
    }

    pub fn get_value(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>) -> Result<(f32, f32), ErrorBM1383AGLV> {

        if !self.enable {
            return Err(ErrorBM1383AGLV::NotInitialized);
        }

        let mut val :[u8; 5] = [0; 5];
        self.get_rawval(i2c, &mut val)?;

        if val[0] == 0 && val[1] == 0 && val[2] == 0 && val[3] == 0 && val[4] == 0 {
            return Err(ErrorBM1383AGLV::NoData);
        }

        let value : [u32; 5] = [val[0] as u32, val[1] as u32, val[2] as u32, val[3] as u32, val[4] as u32];

        let rawpress: u32 = ((value[0] * 256 * 256) + (value[1] * 256) + value[2]) / 4;
        let press = (rawpress as f32) / 2048.0;

        let rawtemp = (value[3] * 256) + value[4];
        let temp = (rawtemp as f32) / 32.0;

        Ok((temp, press))
    }

    fn get_rawval(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, data: &mut [u8]) -> Result<(), ErrorBM1383AGLV> {
        match i2c.write_read(0x5D, &[0x1A], data) {
            Ok(_) => Ok(()),
            _ => Err(ErrorBM1383AGLV::ReadFailure)
        }
    }

    fn write_single(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, memory_address: u8, data: u8) -> Result<(), ErrorBM1383AGLV> {
        let send_data :[u8; 2] = [memory_address, data];
        match i2c.write(0x5D, &send_data) {
            Ok(_) => Ok(()),
            _ => Err(ErrorBM1383AGLV::WriteFailure)
        }
    }

    fn read_single(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, memory_address: u8) -> Result<u8, ErrorBM1383AGLV> {
        let mut recv_data: [u8; 1] = [0];
        match i2c.write_read(0x5D, &[memory_address], &mut recv_data) {
            Ok(_) => Ok(recv_data[0]),
            _ => Err(ErrorBM1383AGLV::ReadFailure)
        }
    }
}

