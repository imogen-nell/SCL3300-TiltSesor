use std::io::{Write, Read};
use std::thread::sleep;
use std::thread;
use std::error::Error;
use std::time::Duration;
use std::mem;
use rppal::gpio::{Gpio, OutputPin};
use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};

const CS_TILT: u8 = 18; // pin12 is BCM 18

const SW_RESET: &[u8] = &[0xB4, 0x00, 0x20, 0x98];
const WHOAMI: &[u8] = &[0x40, 0x00, 0x00, 0x91];
const READ_STAT: &[u8] = &[0x18, 0x00, 0x00, 0xE5];
const MODE_1: &[u8] = &[0xB4, 0x00, 0x00, 0x1F];
const READ_CMD: &[u8] = &[0x34, 0x00, 0x00, 0xDF];
const WAKE_UP: &[u8] = &[0xB4, 0x00, 0x00, 0x1F];
const ANG_CTRL: &[u8] = &[0xB0, 0x00, 0x1F, 0x6F];
const READ_CURR_BANK: &[u8] = &[0x7C, 0x00, 0x00, 0xB3];
const SW_TO_BNK0: &[u8] = &[0xFC, 0x00, 0x00, 0x73];
const ANG_X: &[u8] = &[0x24, 0x00, 0x00, 0xC7];
const ANG_Y: &[u8] = &[0x28, 0x00, 0x00, 0xCD];
const ANG_Z: &[u8] = &[0x2C, 0x00, 0x00, 0xCB];

const BUS: u8 = 1;
const DEV: u8 = 0;

//Functions
fn start_up(spi: &mut Spidev, cs: &mut OutputPin) -> Result<(), Box<dyn Error>> {
    println!("***** start up sequence *****");
    cs.set_high();
    spi.write(SW_TO_BNK0).unwrap();

    let resp1 = frame(spi, cs, SW_RESET)?;
    let resp2 = frame(spi, cs, MODE_1)?;
    let resp3 = frame(spi, cs, ANG_CTRL)?;
    let resp4 = frame(spi, cs, READ_STAT)?;
    let status = read(spi, cs)?;

    //println!("Status: [{}]", status.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
   // println!("Data type of resp1[3]: {:?}", std::any::type_name_of_val(&resp1[3]));

    println!("SW_toBNK0 : [{}]", resp1.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));

    println!("SW RESET  : [{}]", resp2.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    println!("MODE 1    : [{}]", resp3.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    println!("ANG CTRL  : [{}]", resp4.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    println!("READ STAT : [{}]", status.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));

    let crc1 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp1)));
    let crc2 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp2)));
    let crc3 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp3)));
    let crc4 = format!("{:02X}", calculate_crc(bytes_to_u32(&resp4)));
    let crc5 = format!("{:02X}", calculate_crc(bytes_to_u32(&status)));
    // println!("resp1: {:?}", resp1);
    // println!("CRC1: {}", bytes_to_u32(&resp1));

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

    if format!("{:02X}", status[3]) != crc5 {
        println!("Status Checksum error:");
        println!("status[3]: {}", format!("{:02X}", status[3]));
        println!("calculated CRC: {}", crc5);
    }

    sleep(Duration::from_millis(25));
    println!("*****start up sequence complete*****");

    Ok(())
}

fn calculate_crc(data: u32) -> u8 {
    let mut crc: u8 = 0xFF;
    for bit_index in (8..=31).rev() {
        let bit_value: u8 = ((data >> bit_index) & 0x01) as u8;
        crc = crc8(bit_value, crc);
    }
    !crc
}

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

fn bytes_to_u32(data: &[u8]) -> u32 {
    let mut result: u32 = 0;
    for &byte in data {
        result <<= 8; // Shift the current value left by 8 bits
        result |= byte as u32; // Bitwise OR operation to append the byte to the result
    }
    result
}



// Read bytes from the SPI device
// return: vector of bytes read
fn read(spi: &mut Spidev, cs: &mut OutputPin) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut ret = vec![0u8; 4]; // Create a new Vec<u8> to hold the read data
    cs.set_low();
    spi.read(&mut ret)?;
    std::thread::sleep(std::time::Duration::from_millis(20));
    cs.set_high();
   // std::thread::sleep(std::time::Duration::from_millis(15));
    //println!("read: [{}]", ret.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    Ok(ret) // Return the owned Vec<u8>
}

fn write(spi: &mut Spidev, cs: &mut OutputPin, data: &[u8]) {
    cs.set_low();
    spi.write(data);
    sleep(Duration::from_millis(20)); // Must give it at least 10ms to process
    cs.set_high();
    //sleep(Duration::from_millis(15));
}

// Performs write and read, the read will 
// be response to previous request as per the protocol
// arg: request -  bytes to write eg [0x00, 0x00, 0x00, 0x00]
// return: bytes read
fn frame(spi: &mut Spidev, cs: &mut OutputPin, request: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    cs.set_low();
    spi.write(request);
    let mut response = [0u8; 4];
    spi.read(&mut response)?;
    std::thread::sleep(std::time::Duration::from_millis(40));
    cs.set_high();
    //std::thread::sleep(std::time::Duration::from_millis(5));
    Ok(response.to_vec())
}

//

fn main() -> Result<(), Box<dyn Error>> {
    //set up spi device
    let mut spi = Spidev::open(format!("/dev/spidev{}.{}", BUS, DEV)).unwrap();
    let options = SpidevOptions::new()
        .max_speed_hz(2_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).expect("SPI configuration failed");

    let mut cs = Gpio::new()?.get(CS_TILT)?.into_output();
    cs.set_high();
    thread::sleep(Duration::from_millis(50));
    //finidh spi setup
    //start up sequence
    start_up(&mut spi, &mut cs)?;
    //finish start up sequence

    Ok(())
}   
