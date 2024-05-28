use std::io::{Write, Read};
use std::error::Error;
use std::str;
use std::borrow::Cow;
use std::time::Duration;
use std::thread::sleep;
use rppal::gpio::{Gpio, OutputPin};
use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::MutexGuard;

pub struct TiltSensor {
    spi: Spidev,
    cs: OutputPin,
    X_ANG: f64,
    Y_ANG: f64,
    Z_ANG: f64,
}

//Default implementation for TiltSensor
impl TiltSensor {
    const SW_RESET: &[u8] = &[0xB4, 0x00, 0x20, 0x98];
    const WHOAMI: &[u8] = &[0x40, 0x00, 0x00, 0x91];
    const READ_STAT: &[u8] = &[0x18, 0x00, 0x00, 0xE5];
    const MODE_1: &[u8] = &[0xB4, 0x00, 0x00, 0x1F];
    const WAKE_UP: &[u8] = &[0xB4, 0x00, 0x00, 0x1F];
    const ANG_CTRL: &[u8] = &[0xB0, 0x00, 0x1F, 0x6F];
    const SW_TO_BNK0: &[u8] = &[0xFC, 0x00, 0x00, 0x73];
    const ANG_X: &[u8] = &[0x24, 0x00, 0x00, 0xC7];
    const ANG_Y: &[u8] = &[0x28, 0x00, 0x00, 0xCD];
    const ANG_Z: &[u8] = &[0x2C, 0x00, 0x00, 0xCB];

    //TODO: decide where to confgure cs(outside)
    pub fn new(spi: Spidev, cs: OutputPin) -> Self {
        let mut ts = TiltSensor {
            spi,
            cs,
            X_ANG: 0.0,
            Y_ANG: 0.0,
            Z_ANG: 0.0,
        };
        ts.start_up();
        println!("Tilt Sensor initialized");
        ts
    }

    fn start_up(&mut self)-> Result<(), Box<dyn Error>> {
        // Initialize the sensor
        println!("****** start up sequence ******");
        self.cs.set_high();
        sleep(Duration::from_millis(15));
        ///Initial request
        ///No data can be read in this frame
        self.cs.set_low();
        self.spi.write(Self::WAKE_UP).unwrap();
        self.cs.set_low();
        sleep(Duration::from_millis(15));
        self.cs.set_high();
        ///Start-up Sequence
        let resp = self.frame(Self::SW_TO_BNK0);
        sleep(Duration::from_millis(1));
        let resp1 = self.frame(Self::SW_RESET)?;
        sleep(Duration::from_millis(1));
        let resp2 = self.frame(Self::MODE_1)?;
        let resp3 = self.frame(Self::ANG_CTRL)?;
        sleep(Duration::from_millis(25));
        let resp4 = self.frame(Self::READ_STAT)?;
        let resp5 = self.frame(Self::WHOAMI)?;
        let whoami = self.frame(Self::WHOAMI);
    
        ///Print Startu-up sequence results
        println!("SW_toBNK0 : [{}]", resp1.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
        println!("SW RESET  : [{}]", resp2.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
        println!("MODE 1    : [{}]", resp3.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
        println!("ANG CTRL  : [{}]", resp4.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
        println!("READ STAT : [{}]", resp5.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    
        // ///Checksum Calculations to enure startup was successful
        let crc1 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp1)));
        let crc2 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp2)));
        let crc3 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp3)));
        let crc4 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp4)));
        let crc5 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp5)));

        if format!("{:02X}", resp1[3]) != crc1 {
            println!("SW_TO_BNK_0 Checksum error:");
            println!("resp1[3]: {}", format!("{:02X}", resp1[3]));
            println!("calculated CRC: {}", crc1);
        }
        if format!("{:02X}", resp2[3]) != crc2 {
            println!("SW_RESET Checksum error:");
            println!("resp1[3]: {}", format!("{:02X}", resp2[3]));
            println!("calculated CRC: {}", crc2);
        }
        if format!("{:02X}", resp3[3]) != crc3 {
            println!("MODE_1 Checksum error:");
            println!("resp1[3]: {}", format!("{:02X}", resp3[3]));
            println!("calculated CRC: {}", crc3);
        }
        if format!("{:02X}", resp4[3]) != crc4 {
            println!("ANG_CTRL Checksum error:");
            println!("resp1[3]: {}", format!("{:02X}", resp4[3]));
            println!("calculated CRC: {}", crc4);
        }
        if format!("{:02X}", resp5[3]) != crc5 {
            println!("READ_STAT Checksum error:");
            println!("resp1[3]: {}", format!("{:02X}", resp5[3]));
            println!("calculated CRC: {}", crc5);
        }
    
        sleep(Duration::from_millis(25));
        println!("*****start up sequence complete*****");
        
        let data = match whoami {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                return Err(err.into());
            }
        };
        let slice = data.as_slice();
        
        let num = i64::from_str_radix(&format!("{:X}", slice[0]), 16).unwrap();
        let rs = (num & 0b11) as u8;
        let crc = format!("{:02X}", calculate_crc(bytes_to_u32(slice)));
        if crc != format!("{:02X}", slice[3]) {
            println!("Error: Checksum error");
        }
        if rs != 1 {
            println!("Error: Startup Sequence failed");
        }
        Ok(())
    }


    pub fn read_x(&mut self) -> f64 {
        // Read the x-axis value
        self.update_angles();
        self.X_ANG
    }

    pub fn read_y(&mut self) -> f64 {
        // Read the y-axis value
        self.update_angles();
        self.Y_ANG
    }

    pub fn read_z(&mut self) -> f64 {
        // Read the z-axis value
        self.update_angles();
        self.Z_ANG
    }

    fn update_angles(&mut self) {
        // Update all the angles
        let x = self.execute_angle(Self::ANG_X);
        let y = self.execute_angle(Self::ANG_Y);
        let z = self.execute_angle(Self::ANG_Z);
        self.X_ANG = x.unwrap();
        self.Y_ANG = y.unwrap();
        self.Z_ANG = z.unwrap();
    }

    ///Excecutes an angle command, returns the angle read
    ///Argument command: 4-byte command to write to the sensor: ANG_X, ANG_Y, or ANG_Z
    ///Returns: angle in degrees from -90 to 90
    fn execute_angle(&mut self, command: &[u8]) -> Option<f64> {
        //write in previous frame to ensure no garbage values 
        self.cs.set_low();
        self.spi.write(command);
        sleep(Duration::from_millis(20)); // Must give it at least 10ms to process
        self.cs.set_high();
        
        let resp = match self.frame(command) {
            Ok(data) => data,
            Err(_) => {
                println!("Error: failed to get responce");
                return None;
            }
        };
        
        if resp[3] as u8 != calculate_crc(bytes_to_u32(&resp)) {
            println!("checksum error");
            return None;
        }

        let angle = angle_conversion(resp);
        Some(angle)
        
    }

    /// Performs write and read, the data read will 
    /// be response to previous request as per the protocol
    /// arg: request -  bytes to write eg [0x00, 0x00, 0x00, 0x00]
    /// return: bytes read from the device 
    fn frame(&mut self, request: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        self.cs.set_low();
        self.spi.write(request)?;
        sleep(Duration::from_millis(10));
        let mut response = vec![0u8; 4];
        self.spi.read(&mut response)?;
        sleep(Duration::from_millis(10));
        self.cs.set_high();
        Ok(response)
    }

}

///Fucntion used by calcualte_crc()
fn crc8(bit_value: u8, mut crc: u8) -> u8 {
    let mut temp = crc & 0x80;
    if bit_value == 0x01 {
        temp ^= 0x80;
    }
    crc <<= 1;
    if temp > 0 {
        crc ^= 0x1D;
    }
    crc
}


///Calculates checksum for given data bytes
/// Argument data: 32-bit / 4-byte data read from sensor
/// Returns: 8-bit checksum
fn calculate_crc(data: u32) -> u8 {
    let mut crc: u8 = 0xFF;
    for bit_index in (8..=31).rev() {
        let bit_value: u8 = ((data >> bit_index) & 0x01) as u8;
        crc = crc8(bit_value, crc);
    }
    !crc
}

///Converts a slice of bytes to a 32-bit unsigned integer
fn bytes_to_u32(data: &[u8]) -> u32 {
    let mut result: u32 = 0;
    for &byte in data {
        result <<= 8; // Shift the current value left by 8 bits
        result |= byte as u32; // Bitwise OR operation to append the byte to the result
    }
    result
}

///converts data read from spi device to angle
///Argument data: 4-byte data read from sensor
///Returns: angle in degrees from -90 to 90
fn angle_conversion(data: Vec<u8>) -> f64 {
    let val_unsig = u16::from_le_bytes([data[0], data[1]]) as f64;
    let angle = (((val_unsig / 2_i16.pow(14) as f64) * 90.0) * 100.0).round() / 100.0;
    if angle > 90.0 {
        return ((angle - 360.0) * 100.0).round() / 100.0;
    }
    angle
}