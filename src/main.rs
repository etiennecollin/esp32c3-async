#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{Level, Output},
    timer::timg::TimerGroup,
};

use {esp_backtrace as _, esp_println as _};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);
    info!("Embassy initialized!");

    panic!("VERIFY THE GPIO PIN NUMBER IS CONNECTED TO A LED BEFORE RUNNING THIS EXAMPLE!");
    let led = Output::new(peripherals.GPIO20, Level::Low);
    spawner.spawn(blinky(led)).ok();

    spawner.spawn(greet()).ok();

    loop {
        info!("bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}

#[embassy_executor::task]
async fn blinky(mut led: esp_hal::gpio::Output<'static>) {
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn greet() {
    loop {
        info!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
