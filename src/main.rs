use std::io::{Write, Read};
use std::thread::sleep;
use std::thread;
use std::error::Error;
use std::str;
use std::time::Duration;
use std::mem;
use rppal::gpio::{Gpio, OutputPin};
use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};

mod tiltsensor;
const CS_TILT: u8 = 18; // pin12 is BCM 18


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
    //configure cs pin
    let mut cs = Gpio::new()?.get(CS_TILT)?.into_output();
    cs.set_high();
    sleep(Duration::from_millis(50));

    //initialize tilt sensor
    let mut tilt = tiltsensor::TiltSensor::new(spi, cs);
    loop{
        sleep(Duration::from_millis(500));
        println!("*********************");
        println!("X: {} deg", tilt.read_x());
        println!("Y: {} deg", tilt.read_y());
        println!("Z: {} deg", tilt.read_z());
        println!("*********************");
    }

    Ok(())
}   
