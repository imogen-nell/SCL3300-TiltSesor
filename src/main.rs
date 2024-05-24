use std::io;
use std::thread;
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
    //

    println!("Hello, world!");
    Ok(())
}   
