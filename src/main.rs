use rppal::gpio::Gpio;
use simple_logger::SimpleLogger;
use spidev::{SpiModeFlags, Spidev, SpidevOptions};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use scl3300_tiltsensor::tiltsensor;

use std::fs::OpenOptions;
use std::io::Write;

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

    //use simple_logger to log messages
    SimpleLogger::new().init().unwrap();

    //initialize tilt sensor
    let tilt = tiltsensor::TiltSensor::new(spi, cs)?;
    let thread = tilt.spawn_to_thread()?;

    loop {
        if let Some(data) = thread.try_iter().last() {
            println!("*********************");
            println!("X: {} deg", data[0]);
            println!("Y: {} deg", data[1]);
            println!("Z: {} deg", data[2]);
            println!("*********************");
        }
        sleep(Duration::from_millis(500));
    }

}
