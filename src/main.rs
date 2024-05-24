use std::io::Write;
use std::thread::sleep;
use std::error::Error;
use std::time::Duration;
use rppal::gpio::{Gpio, Trigger};
use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};

const CS_TILT: u8 = 18; // pin12 is BCM 18

const SW_RESET: [u8; 4] = [0xB4, 0x00, 0x20, 0x98];
const WHOAMI: [u8; 4] = [0x40, 0x00, 0x00, 0x91];
const READ_STAT: [u8; 4] = [0x18, 0x00, 0x00, 0xE5];
const MODE_1: [u8; 4] = [0xB4, 0x00, 0x00, 0x1F];
const READ_CMD: [u8; 4] = [0x34, 0x00, 0x00, 0xDF];
const WAKE_UP: [u8; 4] = [0xB4, 0x00, 0x00, 0x1F];
const ANG_CTRL: [u8; 4] = [0xB0, 0x00, 0x1F, 0x6F];
const READ_CURR_BANK: [u8; 4] = [0x7C, 0x00, 0x00, 0xB3];
const SW_TO_BNK0: [u8; 4] = [0xFC, 0x00, 0x00, 0x73];
const ANG_X: [u8; 4] = [0x24, 0x00, 0x00, 0xC7];
const ANG_Y: [u8; 4] = [0x28, 0x00, 0x00, 0xCD];
const ANG_Z: [u8; 4] = [0x2C, 0x00, 0x00, 0xCB];

const BUS: u8 = 1;
const DEV: u8 = 0;

//Functions
fn read_start_up() {
    println!("*****(read) start up sequence *****");
    cs.set_high();
    // Request 1
    write(SW_TO_BNK0);

    // Request 2 & response to 1
    let resp1 = frame(SW_RESET);

    // Request 3 SET MEASUREMENT MODE
    let resp2 = frame(MODE_1);

    // Request 4 write ANG_CTRL to enable angle outputs
    let resp3 = frame(ANG_CTRL);

    // Request 5 clear and read STATUS
    let resp4 = frame(READ_STAT);

    // Response to request 5
    let status = read(4);

    println!("status: {:?}", &status);

    println!("SW TO BNK 0 : {:?}", &resp1);
    if format!("{:X}", resp1[3]) != calculate_crc(&resp1) {
        println!("checksum error resp1");
    }
    println!("SW RESET    : {:?}", &resp2);
    if format!("{:X}", resp2[3]) != calculate_crc(&resp2) {
        println!("checksum error resp2");
    }
    println!("MODE 1      : {:?}", resp3);
    if format!("{:X}", resp3[3]) != calculate_crc(&resp3) {
        println!("checksum error resp3");
    }
    println!("ANG CTRL    : {:?}", &resp4);
    if format!("{:X}", resp4[3]) != calculate_crc(&resp4) {
        println!("checksum error resp4");
    }
    println!("READ STAT   : {:?}", &status);
    if format!("{:X}", status[3]) != calculate_crc(&status) {
        println!("checksum error status");
    }
    sleep(Duration::from_millis(25));
    println!("*****start up sequence complete*****");
}

fn calculate_crc(data: u32) -> u8 {
    let mut crc: u8 = 0xFF;
    for bit_index in (8..=31).rev() {
        let bit_value = ((data >> bit_index) & 0x01) as u8;
        crc = crc8(bit_value, crc);
    }
    !crc
}

fn crc8(bit_value: u8, mut crc: u8) -> u8 {
    let temp = crc & 0x80;
    if bit_value == 0x01 {
        crc ^= 0x80;
    }
    crc <<= 1;
    if temp > 0 {
        crc ^= 0x1D;
    }
    crc
}

// Read bytes from the SPI device
// arg: bytecount - number of bytes to read eg 4
// return: vector of bytes read
fn read(bytecount: usize) -> Vec<u8> {
    cs.set_low();
    let mut ret = vec![0; bytecount];
    spi.read(&mut ret).unwarp();
    std::thread::sleep(std::time::Duration::from_millis(20));
    cs.set_high();
    std::thread::sleep(std::time::Duration::from_millis(15));
    ret
}

// Performs write and read, the read will 
// be response to previous request as per the protocol
// arg: request - vector of bytes to write eg [0x00, 0x00, 0x00, 0x00]
// arg: bytecount - number of bytes to read eg 4
// return: vector of bytes read
fn frame(request: &[u8], bytecount: usize) -> Vec<u8> {
    cs.set_low();
    spi.write(request);
    let mut response = vec![0; bytecount];
    spi.read(&mut response);
    std::thread::sleep(std::time::Duration::from_millis(40));
    cs.set_high();
    std::thread::sleep(std::time::Duration::from_millis(5));
    response
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



    println!("Hello, world!");
    Ok(())
}   
