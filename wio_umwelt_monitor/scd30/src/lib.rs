//! for CO2 Sensor

#![no_std]

use panic_halt as _;
use wio_terminal as wio;
use wio_terminal::prelude::*;
use wio::hal::gpio::*;
use wio::hal::sercom::*;


pub struct SCD30 {
    scd30_address: u8
}

impl SCD30 {
    pub fn new() -> SCD30 {
        SCD30 {
            scd30_address: 0x61
        }
    }

    pub fn init(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, interval: u16) -> Result<(), ()> {

        // 2 seconds between measurements
        self.set_measurement_interval(i2c, interval)?;

        // start periodic measuments
        self.start_periodic_measurment(i2c)
    }

    pub fn set_measurement_interval(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, interval: u16) -> Result<(), ()> {
        let scd30_set_measurement_interval: u16 = 0x4600;
        self.write_command(i2c, scd30_set_measurement_interval, interval)
    }

    pub fn start_periodic_measurment(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>) -> Result<(), ()> {
        let scd30_continuous_measurement: u16 = 0x0010;
        self.write_command(i2c, scd30_continuous_measurement, 0x0000)
    }

    pub fn is_available(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>) -> Result<bool, ()> {
        let mut data: [u8; 2] = [0, 0];

        if let Err(_) = i2c.write(self.scd30_address, &[0x02, 0x02]) {
            return Err(());
        }

        if let Err(_) = i2c.read(self.scd30_address, &mut data) {
            return Err(());
        }

        if data[0] != 0 || data[1] != 0 {
            Ok(true)
        }
        else {
            Ok(false)
        }
    }

    pub fn set_auto_calibration(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, enable: bool) -> Result<(), ()> {
        if enable {
            self.write_command(i2c, 0x5306, 1)
        }
        else {
            self.write_command(i2c, 0x5306, 0)
        }
    }

    pub fn get_value(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>) -> Result<(f32, f32, f32), ()> {
        let mut buf: [u8; 18] = [0; 18];

        if let Err(_) = i2c.write(self.scd30_address, &[0x03, 0x00]) {
            return Err(());
        }

        if let Err(_) = i2c.read(self.scd30_address, &mut buf) {
            return Err(());
        }

        let data: [u32; 12] = [
            buf[0]  as u32, buf[1]  as u32, buf[3]  as u32, buf[4]  as u32,
            buf[6]  as u32, buf[7]  as u32, buf[9]  as u32, buf[10] as u32,
            buf[12] as u32, buf[13] as u32, buf[15] as u32, buf[16] as u32
        ];

        let co2 : u32 =(data[0] << 24) | (data[1] << 16) | (data[2]  << 8) | data[3];
        let tmp: u32 =(data[4] << 24) | (data[5] << 16) | (data[6]  << 8) | data[7];
        let hum : u32 =(data[8] << 24) | (data[9] << 16) | (data[10] << 8) | data[11];

        Ok((self.convert_bin2float(co2), self.convert_bin2float(tmp), self.convert_bin2float(hum)))
    }

    fn convert_bin2float(&mut self, data: u32) -> f32 {
        let sign = data >> 31;
        let exponent = (data & 0x7F800000) >> 23;
        let mut fraction = data & 0x007FFFFF;

        if exponent != 0 {
            fraction |= 0x00800000;
        }

        let mut converted = fraction as f32;

        for _ in 0.. (127 - exponent + 23) {
            converted /= 2.0;
        }

        if sign == 1 {
            converted *= -1.0;
        }

        converted
    }

    pub fn stop_measurement(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>) -> Result<(), ()> {
        match i2c.write(self.scd30_address, &[0x01, 0x04]) {
            Ok(_) => Ok(()),
            _ => Err(())
        }
    }

    fn write_command(&mut self, i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>>, command: u16, arguments: u16) -> Result<(), ()> {

        let crc = self.calculate_crc(arguments);
        let buf :[u8; 5] = [(command >> 8) as u8, (command & 0x00ff) as u8, (arguments >> 8) as u8, (arguments & 0x00ff) as u8, crc];

        match i2c.write(self.scd30_address, &buf) {
            Ok(_) => Ok(()),
            _ => Err(())
        }
    }

    fn calculate_crc(&mut self, arguments: u16) -> u8 {
        let mut crc = 0xffu8;
        let scd30_polynomial: u8 = 0x31;
        let data :[u8; 2] = [(arguments >> 8) as u8, (arguments & 0x00ff) as u8];

        for i in 0..2 {
            // calculates 8-Bit checksum with given polynomial
            crc ^= data[i];

            for _ in 0..8 {
                if (crc & 0x80u8) != 0x00u8 {
                    crc = (crc << 1) ^ scd30_polynomial;
                }
                else {
                    crc = crc << 1;
                }
            }
        }
        crc
    }
}

