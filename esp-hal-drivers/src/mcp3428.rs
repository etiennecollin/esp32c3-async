//! # mcp3428
//!
//! Inspired by: https://github.com/dbrgn/mcp3425-rs/
//!
//! ## Overview
//!
//! This driver provides an abstraction to interact with the MCP3428 ADC.
//!
//! ## Example
//!
//! ```rust,ignore
//! use embassy_time::{Duration, Timer};
//! use esp_hal::i2c::master::{Config, I2c};
//!
//! // Prepare the I2C peripheral
//! let peripherals = esp_hal::init(esp_hal::Config::default());
//! let i2c = I2c::new(peripherals.I2C0, Config::default())
//!     .unwrap()
//!     .with_sda(peripherals.GPIO9)
//!     .with_scl(peripherals.GPIO8)
//!     .into_async();
//!
//! // Generate the configuration
//! let address = 0x68;
//! let mut config = ThermostatConfig::new(address, i2c, Mode::OneShot)
//!     .with_gain(Gain::Gain1)
//!     .with_resolution(Resolution::Bits12Sps240);
//!
//! // Read channel 1 and channel 2 in one-shot mode
//! config.set_channel(Channel::Channel1);
//! let voltage_1 = config.one_shot_measurement().await.ok();
//! config.set_channel(Channel::Channel2);
//! let voltage_2 = config.one_shot_measurement().await.ok();
//! println!("Voltage 1: {}", voltage_1);
//! println!("Voltage 2: {}", voltage_2);
//!
//! // Prepare the configuration for continuous reading of channel 1
//! config.set_channel(Channel::Channel1);
//! config.set_mode(Mode::Continuous);
//! config.write_config().await.ok();
//!
//! // Read the measurement in a loop
//! loop {
//!     // Read every second
//!     Timer::after(Duration::from_millis(1_000)).await;
//!
//!     // Read the measurement
//!     let voltage = config.get_measurement().await.ok();
//!     println!("Voltage: {}", voltage);
//! }
//! ```

use embassy_time::{Duration, Timer};
use esp_hal::{i2c::master::I2c, Async};

pub struct ThermostatConfig {
    address: u8,
    mode: Mode,
    i2c: I2c<'static, Async>,
    resolution: Resolution,
    gain: Gain,
    channel: Channel,
}

#[allow(unused, dead_code)]
impl ThermostatConfig {
    pub fn new(address: u8, i2c: I2c<'static, Async>, mode: Mode) -> Self {
        Self {
            address,
            mode,
            i2c,
            resolution: Resolution::default(),
            gain: Gain::default(),
            channel: Channel::default(),
        }
    }

    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_gain(mut self, gain: Gain) -> Self {
        self.gain = gain;
        self
    }

    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    pub fn set_channel(&mut self, channel: Channel) {
        self.channel = channel;
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    fn get_sleep_ms(&self) -> u64 {
        match self.resolution {
            Resolution::Bits12Sps240 => 4,
            Resolution::Bits14Sps60 => 15,
            Resolution::Bits16Sps15 => 57,
        }
    }

    fn config_flag(&self) -> u8 {
        self.channel.bits() | self.resolution.bits() | self.gain.bits()
    }

    fn command(&self) -> u8 {
        match self.mode {
            Mode::OneShot => ConfigRegister::NOT_READY | self.mode.bits() | self.config_flag(),
            Mode::Continuous => self.mode.bits() | self.config_flag(),
        }
    }

    pub async fn one_shot_measurement(&mut self) -> Result<i32, Error> {
        if self
            .i2c
            .write(self.address, &[self.command()])
            .await
            .is_err()
        {
            return Err(Error::I2c);
        }
        Timer::after(Duration::from_millis(self.get_sleep_ms() + 2)).await;

        let voltage = self.get_measurement().await?;
        Ok(voltage)
    }

    pub async fn write_config(&mut self) -> Result<(), Error> {
        // Prepare to read channel 1
        if self
            .i2c
            .write(self.address, &[self.command()])
            .await
            .is_err()
        {
            return Err(Error::I2c);
        };
        Timer::after(Duration::from_millis(self.get_sleep_ms())).await;

        // Poll until ready
        let mut buf = [0u8; 3];
        loop {
            if self.i2c.read(self.address, &mut buf).await.is_err() {
                return Err(Error::I2c);
            }
            let config_reg = ConfigRegister::new(ConfigRegister::ALL & buf[2]);

            if config_reg.is_ready() {
                return Ok(());
            } else {
                // Not yet ready, wait some more time
                Timer::after(Duration::from_millis(1)).await;
            }
        }
    }

    pub async fn get_measurement(&mut self) -> Result<i32, Error> {
        loop {
            // Read measurement and config register
            let (measurement, config_reg) = self.read_i2c().await?;

            // Check "Not Ready" flag. See datasheet section 5.1.1 for more details.
            if config_reg.is_ready() {
                // Calculate voltage from raw value
                let voltage = self.calculate_voltage(measurement)?;
                return Ok(voltage);
            } else {
                // Not yet ready, wait some more time
                Timer::after(Duration::from_millis(1)).await;
            }
        }
    }

    async fn read_i2c(&mut self) -> Result<(i16, ConfigRegister), Error> {
        let mut buf = [0u8; 3];
        if self.i2c.read(self.address, &mut buf).await.is_err() {
            return Err(Error::I2c);
        }
        let measurement = i16::from_be_bytes([buf[0], buf[1]]);
        let config_reg = ConfigRegister::new(buf[2] & ConfigRegister::ALL);
        Ok((measurement, config_reg))
    }

    /// Calculate the voltage in mV for the measurement result at the specified sample rate.
    ///
    /// If the value is a saturation value, an error is returned.
    fn calculate_voltage(&self, measurement: i16) -> Result<i32, Error> {
        // Handle saturation / out of range values
        if measurement == self.resolution.max() {
            return Err(Error::VoltageTooHigh);
        } else if measurement == self.resolution.min() {
            return Err(Error::VoltageTooLow);
        }

        Ok(measurement as i32 * (REF_MILLIVOLTS * 2) as i32 / (1 << self.resolution.res_bits()))
    }
}

/// ADC reference voltage: +-2048mV
const REF_MILLIVOLTS: i16 = 2048;

/// All possible errors in this crate
#[allow(unused, dead_code)]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// I2C bus error
    I2c,
    /// Voltage is too high to be measured.
    VoltageTooHigh,
    /// Voltage is too low to be measured.
    VoltageTooLow,
    /// A measurement in continuous mode has been triggered without previously
    /// writing the configuration to the device.
    NotInitialized,
    /// A measurement returned a stale result.
    ///
    /// In continuous mode, this can happen if you poll faster than the sample
    /// rate. See datasheet section 5.1.1 for more details.
    ///
    /// In one-shot mode, this is probably a timing bug that should be reported to
    /// <https://github.com/dbrgn/mcp3425-rs/issues/>!
    ///
    NotReady,
}

pub struct ConfigRegister {
    pub value: u8,
}

impl ConfigRegister {
    pub const NOT_READY: u8 = 0b10000000;
    pub const MODE: u8 = 0b00010000;
    pub const SAMPLE_RATE_H: u8 = 0b00001000;
    pub const SAMPLE_RATE_L: u8 = 0b00000100;
    pub const GAIN_H: u8 = 0b00000010;
    pub const GAIN_L: u8 = 0b00000001;
    pub const ALL: u8 = Self::NOT_READY
        | Self::MODE
        | Self::SAMPLE_RATE_H
        | Self::SAMPLE_RATE_L
        | Self::GAIN_H
        | Self::GAIN_L;

    pub fn new(value: u8) -> Self {
        Self { value }
    }

    /// Whether all set bits in NOT_READY flags value are also set in the value.
    pub fn is_ready(&self) -> bool {
        (self.value & Self::NOT_READY) != Self::NOT_READY
    }
}

#[allow(unused, dead_code)]
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mode {
    OneShot = 0b00000000,
    Continuous = 0b00010000,
}

impl Mode {
    pub fn bits(&self) -> u8 {
        *self as u8
    }
}

/// Conversion bit resolution and sample rate
///
/// * 15 SPS -> 16 bits
/// * 60 SPS -> 14 bits
/// * 240 SPS -> 12 bits
///
/// Defaults to 12 bits / 240 SPS (`Bits12Sps240`),
/// matching the power-on defaults of the device.
#[allow(unused, dead_code)]
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Resolution {
    /// 16 bits / 15 SPS. This allows you to measure voltage in 62.5 µV steps.
    Bits16Sps15 = 0b00001000,
    /// 14 bits / 60 SPS. This allows you to measure voltage in 250 µV steps.
    Bits14Sps60 = 0b00000100,
    /// 12 bits / 240 SPS. This allows you to measure voltage in 1 mV steps.
    Bits12Sps240 = 0b00000000,
}

impl Resolution {
    /// Return the bitmask for this sample rate.
    pub fn bits(&self) -> u8 {
        *self as u8
    }

    /// Return the number of bits of accuracy this sample rate gives you.
    pub fn res_bits(&self) -> u8 {
        match *self {
            Resolution::Bits16Sps15 => 16,
            Resolution::Bits14Sps60 => 14,
            Resolution::Bits12Sps240 => 12,
        }
    }

    /// Return the maximum output code.
    pub fn max(&self) -> i16 {
        match *self {
            Resolution::Bits16Sps15 => 32767,
            Resolution::Bits14Sps60 => 8191,
            Resolution::Bits12Sps240 => 2047,
        }
    }

    /// Return the minimum output code.
    pub fn min(&self) -> i16 {
        match *self {
            Resolution::Bits16Sps15 => -32768,
            Resolution::Bits14Sps60 => -8192,
            Resolution::Bits12Sps240 => -2048,
        }
    }
}

impl Default for Resolution {
    /// Default implementation matching the power-on defaults of the device.
    fn default() -> Self {
        Resolution::Bits12Sps240
    }
}

/// Programmable gain amplifier (PGA)
///
/// Defaults to no amplification (`Gain1`),
/// matching the power-on defaults of the device.
#[allow(unused, dead_code)]
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Gain {
    /// Amplification factor 1.
    Gain1 = 0b00000000,
    /// Amplification factor 2.
    Gain2 = 0b00000001,
    /// Amplification factor 4.
    Gain4 = 0b00000010,
    /// Amplification factor 8.
    Gain8 = 0b00000011,
}

impl Gain {
    /// Return the bitmask for this gain configuration.
    pub fn bits(&self) -> u8 {
        *self as u8
    }
}

impl Default for Gain {
    /// Default implementation matching the power-on defaults of the device.
    fn default() -> Self {
        Gain::Gain1
    }
}

/// Selected ADC channel
///
/// Defaults to channel 1.
#[allow(unused, dead_code)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Channel {
    /// First channel (Default)
    Channel1 = 0b0000_0000,
    /// Second channel
    ///
    /// Note: Only supported by MCP3426/7/8, and if the `dual_channel` or
    /// `quad_channel` cargo feature is enabled.
    Channel2 = 0b0010_0000,
    /// Third channel
    ///
    /// Note: Only supported by MCP3428, and if the `quad_channel` cargo
    /// feature is enabled.
    Channel3 = 0b0100_0000,
    /// Fourth channel
    ///
    /// Note: Only supported by MCP3428, and if the `quad_channel` cargo
    /// feature is enabled.
    Channel4 = 0b0110_0000,
}

impl Default for Channel {
    fn default() -> Self {
        Self::Channel1
    }
}

impl Channel {
    /// Return the bitmask for this channel configuration.
    pub fn bits(&self) -> u8 {
        *self as u8
    }
}
