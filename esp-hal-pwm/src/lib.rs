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
//! pwm.set_frequency_hz(60).ok();
//!
//! pwm.start(50).ok();
//! ```
//!
//! ## Features
//!
//! - `defmt`: Implement `defmt::Format` on certain types.
//! - `embassy`: Songs and lists of tones are played asynchronously using embassy.
//! - `esp32c3`: Target the ESP32-C3.

#![no_std]
use core::{fmt::Debug, ops::DerefMut};

use esp_hal::{
    clock::Clocks,
    gpio::OutputPin,
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

    FrequencyNotConfigured,
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
pub struct Pwm<'a, O: OutputPin> {
    timer: Timer<'a, LowSpeed>,
    channel_number: channel::Number,
    output_pin: PeripheralRef<'a, O>,
}

impl<'a, O: OutputPin + Peripheral<P = O>> Pwm<'a, O> {
    pub fn new(
        ledc: &'a Ledc,
        timer_number: timer::Number,
        channel_number: channel::Number,
        output_pin: impl Peripheral<P = O> + 'a,
    ) -> Self {
        Self {
            timer: ledc.timer::<LowSpeed>(timer_number),
            channel_number,
            output_pin: output_pin.into_ref(),
        }
    }

    /// Start the PWM.
    ///
    /// # Arguments
    /// - `duty_cycle` - The duty cycle percentage (0-100).
    pub fn start(&mut self, duty_cycle: u8) -> Result<(), Error> {
        if !self.timer.is_configured() {
            return Err(Error::FrequencyNotConfigured);
        }

        let mut channel = Channel::new(self.channel_number, self.output_pin.deref_mut());
        channel.configure(channel::config::Config {
            timer: &self.timer,
            duty_pct: duty_cycle,
            pin_config: channel::config::PinConfig::PushPull,
        })?;

        Ok(())
    }

    /// Start a duty cycle fade from `start` to `end` over `duration` milliseconds.
    ///
    /// # Arguments
    /// - `start` - The starting duty cycle percentage (0-100).
    /// - `end` - The ending duty cycle percentage (0-100).
    /// - `duration` - The duration of the fade in milliseconds.
    pub fn start_duty_fade(&mut self, start: u8, end: u8, duration: u16) -> Result<(), Error> {
        if !self.timer.is_configured() {
            return Err(Error::FrequencyNotConfigured);
        }

        // Make sure the duty cycle is within bounds
        if start > 100 || end > 100 {
            return Err(Error::Channel(channel::Error::Duty));
        }

        let mut channel = Channel::new(self.channel_number, self.output_pin.deref_mut());
        channel.configure(channel::config::Config {
            timer: &self.timer,
            duty_pct: start,
            pin_config: channel::config::PinConfig::PushPull,
        })?;

        channel.start_duty_fade(start, end, duration)?;

        Ok(())
    }

    /// Stop the PWM.
    ///
    /// The duty cycle will be set to 0.
    pub fn stop(&mut self) -> Result<(), Error> {
        if !self.timer.is_configured() {
            return Err(Error::FrequencyNotConfigured);
        }

        let mut channel = Channel::new(self.channel_number, self.output_pin.deref_mut());
        channel.configure(channel::config::Config {
            timer: &self.timer,
            duty_pct: 0,
            pin_config: channel::config::PinConfig::PushPull,
        })?;

        Ok(())
    }

    /// Set the frequency of the PWM.
    ///
    /// # Arguments
    /// - `frequency` - The frequency in Hz.
    pub fn set_frequency_hz(&mut self, frequency: u32) -> Result<(), Error> {
        // If the frequency is 0, stop the PWM
        if frequency == 0 {
            return self.stop();
        }

        // Max duty resolution for a frequency:
        // Integer(log2(LEDC_APB_CKL / frequency))
        // Source: https://github.com/esp-rs/esp-hal-community
        let mut result = 0;
        let mut value = (Clocks::get().apb_clock / frequency).raw();

        // Limit duty resolution to 14 bits
        while value > 1 && result < 14 {
            value >>= 1;
            result += 1;
        }

        self.timer.configure(timer::config::Config {
            duty: timer::config::Duty::try_from(result).unwrap(),
            clock_source: timer::LSClockSource::APBClk,
            frequency: frequency.Hz(),
        })?;

        Ok(())
    }

    /// Get the frequency of the PWM.
    pub fn get_frequency_hz(&self) -> Result<u32, Error> {
        if !self.timer.is_configured() {
            return Err(Error::FrequencyNotConfigured);
        }
        Ok(self.timer.frequency())
    }
}
