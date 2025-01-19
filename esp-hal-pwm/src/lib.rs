//! # PWM
//!
//! ## Overview
//!
//! This driver provides an abstraction over LEDC to drive a PWM signal
//! through a user-friendly API.
//!
//! ## Example
//!
//! ```rust,ignore
//! let peripherals = esp_hal::init(esp_hal::Config::default());
//! let mut ledc = Ledc::new(peripherals.LEDC);
//! let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
//!
//! ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
//!
//! let mut pwm = Pwm::new(
//!     &ledc,
//!     timer::Number::Timer0,
//!     channel::Number::Channel1,
//!     io.pins.gpio6,
//! );
//! pwm.configure();
//!
//! pwm.set_duty_cycle(50).unwrap();
//! ```
//!
//! ## Features
//!
//! - `defmt`: Implement `defmt::Format` on certain types.
//! - `embassy`: Songs and lists of tones are played asynchronously using embassy.
//! - `esp32c3`: Target the ESP32-C3.

#![no_std]
use core::{fmt::Debug, ops::DerefMut};

#[cfg(not(feature = "embassy"))]
use esp_hal::delay::Delay;
use esp_hal::{
    clock::Clocks,
    gpio::{AnyPin, Level, Output, OutputPin, Pin},
    ledc::{
        channel::{self, Channel, ChannelIFace},
        timer::{self, Timer, TimerIFace},
        Ledc, LowSpeed,
    },
    peripheral::{Peripheral, PeripheralRef},
    time::RateExtU32,
};

/// Errors from PWM
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Errors from [channel::Error]
    Channel(channel::Error),

    /// Errors from [timer::Error]
    Timer(timer::Error),
}

/// Converts [channel::Error] into [self::Error]
impl From<channel::Error> for Error {
    fn from(error: channel::Error) -> Self {
        Error::Channel(error)
    }
}

/// Converts [timer::Error] into [self::Error]
impl From<timer::Error> for Error {
    fn from(error: timer::Error) -> Self {
        Error::Timer(error)
    }
}

/// A PWM instance driven by Ledc
pub struct Pwm<'a> {
    timer: Timer<'a, LowSpeed>,
    channel: Channel<'a, LowSpeed>,
}

impl<'a> Pwm<'a> {
    pub fn new(
        ledc: &'a Ledc,
        timer_number: timer::Number,
        channel_number: channel::Number,
        output_pin: AnyPin,
    ) -> Self {
        Self {
            timer: ledc.timer::<LowSpeed>(timer_number),
            channel: ledc.channel(channel_number, output_pin),
        }
    }

    pub fn configure(&mut self) {
        self.timer
            .configure(timer::config::Config {
                duty: timer::config::Duty::Duty14Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: 1.Hz(),
            })
            .unwrap();
        self.channel
            .configure(channel::config::Config {
                timer: &self.timer,
                duty_pct: 100,
                pin_config: channel::config::PinConfig::PushPull,
            })
            .unwrap();
    }

    pub fn set_duty_cycle(&mut self, duty_cycle: u8) -> Result<(), Error> {
        if let Err(e) = self.channel.set_duty(duty_cycle) {
            return Err(Error::Channel(e));
        };

        Ok(())
    }

    pub fn set_frequency(&mut self, frequency: u32) -> Result<(), Error> {
        let res = self.timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: frequency.Hz(),
        });
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Timer(e)),
        }
    }
}
