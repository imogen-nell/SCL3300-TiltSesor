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
    println!("****** start up sequence ******");
    cs.set_high();
    sleep(duration::from_millis(15));
    cs.set_low();
    spi.write(SW_TO_BNK0).unwrap();
    sleep(Duration::from_millis(15));
    cs.set_high();

    let resp = frame(spi, cs, WAKE_UP)?;
    sleep(duration::from_millis(1));
    let resp1 = frame(spi, cs, SW_RESET)?;
    sleep(duration::from_millis(1));
    let resp2 = frame(spi, cs, MODE_1)?;
    let resp3 = frame(spi, cs, ANG_CTRL)?;
    sleep(duration::from_millis(25));
    let resp4 = frame(spi, cs, READ_STAT)?;
    let resp5 = frame(spi, cs, READ_STAT)?;
    let status = frame(spi, cs, s\READ_STAT)?;

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
// Separates the OP code into RW, ADDR, RS and prints them on the screen
// Used by execute_command()
// Argument data: 8-bit / 1-byte hex string e.g., '0xC1'
// 2 LSB - RW, next 5 bits - ADDR, last 1 bit - RS
fn get_op(data: &str) {
    let num = i64::from_str_radix(data.trim_start_matches("0x"), 16).unwrap();
    let num_binary = format!("{:08b}", num);
    
    println!("RW  : {}", &num_binary[0..2]);
    println!("ADDR: {:X}", i64::from_str_radix(&num_binary[2..7], 2).unwrap());
    println!("RS  : {}", &num_binary[7..]);
}

// Executes the command and prints the response
// Argument command: list of 4 bytes to write e.g., ['0x00', '0x00', '0x00', '0x00']
// Argument key: string to print the command name e.g., "WHOAMI"
fn execute_command(spi: &mut Spidev, cs: &mut OutputPin, command: &[u8], key: &str) {
    spi.write(command);
    let frame_result = frame(spi, cs, command);
    let i = match frame_result {
        Ok(data) => data,
        Err(err) => {
            println!("Error: {:?}", err);
            return;
        }
    };
    
    let i_slice = i.as_slice();
    let crc = format!("{:02X}", calculate_crc(bytes_to_u32(i_slice)));
    
    if format!("{:02X}", i_slice[3]) != crc {
        println!("checksum error");
        return;
    } else {
        println!("\n*************************\n");
        println!("{} response:", key);
        get_op(&format!("{:02X}", i_slice[0]));
        println!("Data: [{}]", i_slice[1..3].iter().map(|&b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
        println!("\n*************************\n");
    }
}

fn execute_angle(spi: &mut Spidev, cs: &mut OutputPin, command: &[u8], key: &str) -> Option<f64> {
    //write in previous frame to ensure no garbage values 
    cs.set_low();
    spi.write(command);
    sleep(Duration::from_millis(20)); // Must give it at least 10ms to process
    cs.set_high();
    ///
    let resp = match frame(spi, cs, command) {
        Ok(data) => data,
        Err(_) => {
            println!("Error: failed to get responce");
            return None;
        }
    };
    
    if !key.contains("ANG_") {
        println!("invalid command");
        return None;
    }
    
    if resp[3] as u8 != calculate_crc(bytes_to_u32(&resp)) {
        println!("checksum error");
        return None;
    }
    //println!("Data: [{}]", resp.iter().map(|&b| format!("{:02X}", b)).collect::<Vec<_>>().join(", "));
    
    let angle = anlge_conversion(resp);
    println!("{}: {} deg", key, angle);
    Some(angle)
    
}

fn execute_angles(spi: &mut Spidev, cs: &mut OutputPin){
    //write in previous frame to ensure no garbage values 
    cs.set_low();
    spi.write(ANG_X);
    sleep(Duration::from_millis(11)); // Must give it at least 10ms to process
    cs.set_high();
    //

    let x = match frame(spi, cs, ANG_Y) {
        Ok(data) => data,
        Err(_) => {
            println!("Error: failed to get responce");
            return;
        }
    };
    let y = match frame(spi, cs, ANG_Z) {
        Ok(data) => data,
        Err(_) => {
            println!("Error: failed to get responce");
            return;
        }
    };
    let z = match read(spi, cs) {
        Ok(data) => data,
        Err(_) => {
            println!("Error: failed to get responce");
            return;
        }
    };

    //crc check
    if x[3] as u8 != calculate_crc(bytes_to_u32(&x)) {
        println!("x checksum error");
        return;
    }
    if y[3] as u8 != calculate_crc(bytes_to_u32(&y)) {
        println!("cy hecksum error");
        return;
    }
    if z[3] as u8 != calculate_crc(bytes_to_u32(&z)) {
        println!("z checksum error");
        return;
    }       
    
    println!("X : {} deg", anlge_conversion(x));   
    println!("Y : {} deg", anlge_conversion(y));
    println!("Z : {} deg", anlge_conversion(z)); 
}

fn anlge_conversion(data: Vec<u8>) -> f64 {
    let abs_val = i16::from_le_bytes([data[0], data[1]]) as f64;
    println!("signed value: {}", abs_val);
    println!("unsigned value: {}", u16::from_le_bytes([data[0], data[1]]) as f64);
    let angle = (((abs_val.abs() / 2_i16.pow(14) as f64) * 90.0) * 100.0).round() / 100.0;
    angle
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

    start_up(&mut spi, &mut cs)?;
    execute_command(&mut spi, &mut cs, WHOAMI, "WHOAMI");
    
    ///loooooooping angle readings
    loop {
        println!("********************");
        //execute_angles(&mut spi, &mut cs);
        match execute_angle(&mut spi, &mut cs, ANG_Y, "ANG_Y") {
            Some(angle) => {/*do nothing*/},
            None => println!("Failed to execute angle command"),
        }
        // match execute_angle(&mut spi, &mut cs, ANG_Z, "ANG_Z") {
        //     Some(angle) => {/*do nothing*/},
        //     None => println!("Failed to execute angle command"),
        // }
        println!("********************");
        // thread::sleep(Duration::from_secs(.5));
    }

    Ok(())
}   
