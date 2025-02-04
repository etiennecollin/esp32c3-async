#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{AnyPin, Level, Output},
    timer::timg::TimerGroup,
};

use {esp_backtrace as _, esp_println as _};

#[cfg(feature = "defmt")]
use defmt::info;
#[cfg(feature = "log")]
use log::info;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    info!("Embassy initialized!");
    todo!("VERIFY THE GPIO PIN NUMBER IS CONNECTED TO A LED BEFORE RUNNING THIS EXAMPLE!");
    spawner.spawn(blinky(peripherals.GPIO20.into())).ok();
    spawner.spawn(greet()).ok();
}

#[embassy_executor::task]
async fn blinky(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low);
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(20)).await;
        led.set_low();
        Timer::after(Duration::from_millis(2_000)).await;
    }
}

#[embassy_executor::task]
async fn greet() {
    loop {
        info!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
