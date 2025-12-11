//! # SH8601 Driver Crate
//!
//! An embedded-graphics compatible driver for the SH8601 AMOLED display controller IC.
//!
//! This driver is not embedded-hal compatible, but provides a generic interface
//! for controlling the SH8601 display controller.
//! Different displays can be supported by implementing the `ControllerInterface` and `ResetInterface` traits.
//! This is because the SH8601 is used in different displays with various controller interfaces such as SPI or QSPI.
//! Additionally, the reset pin is controlled via GPIO or I2C GPIO expander.
//!
//! The driver currently incorporates support the Waveshare 1.8" AMOLED display out of the box, but can be extended to support other displays using the SH8601 controller.
//!
//! ## Usage
//!
//! 1. Implement the `ControllerInterface` trait for the controller driving interface Ex. QSPI
//! 2. Implement the `ResetInterface` trait for the Reset pin.
//! 3. Create a `Sh8601Driver` instance with the display interface and reset pin.
//! 4. Use the driver to draw using `embedded-graphics`.
//!
//! If you are going to use heap allocated framebuffer, you will need to make sure that an allocator is available in your environment.
//! In some crates this is done by enabling the `alloc` feature.
//!
//! ## Feature Flags
#![doc = document_features::document_features!()]
//!
//! ## Examples
//!
//! See the `examples` directory for a usage example with the WaveShare 1.8" AMOLED Display.
//!
//! The WaveShare 1.8" AMOLED Display controls the SH8601 via an ESP32-S3 over QSPI and uses an I2C GPIO expander for the reset pin.
//! The example implementation uses a PSRAM heap allocated framebuffer and DMA for efficient transfers.
//!
//! The schematic is available here: <https://files.waveshare.com/wiki/ESP32-S3-Touch-AMOLED-1.8/ESP32-S3-Touch-AMOLED-1.8.pdf>
//!
//! To run the example, with the WaveShare 1.8" AMOLED Display, clone the project and run following command from the project root:
//! ```bash
//! cargo run --example ws_18in_amoled --features "waveshare_18_amoled"
//! ```
//!

#![no_std]
#[cfg(feature = "waveshare_18_amoled")]
pub mod displays;

#[cfg(feature = "waveshare_18_amoled")]
pub use displays::waveshare_18_amoled::*;

extern crate alloc;

mod graphics_core;

use alloc::boxed::Box;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_hal::delay::DelayNs;

/// Configuration for the display dimensions.
#[derive(Debug, Clone, Copy)]
pub struct DisplaySize {
    /// Display width in pixels.
    pub width: u16,
    /// Display height in pixels.
    pub height: u16,
}

impl DisplaySize {
    pub const fn new(width: u16, height: u16) -> Self {
        DisplaySize { width, height }
    }
}

/// SH8601 Driver Errors
#[derive(Debug)]
pub enum DriverError<InterfaceError, ResetError> {
    /// Error originating from the display interface (QSPI/SPI/I2C).
    InterfaceError(InterfaceError),
    /// Error originating from the reset pin control.
    ResetError(ResetError),
    /// Invalid configuration provided to the driver.
    InvalidConfiguration(&'static str),
}

/// Trait to implement the SH8601 controller communication interface (QSPI, SPI, etc.).
pub trait ControllerInterface {
    /// The specific error type for this interface implementation.
    type Error;

    /// Sends a command byte to the display.
    fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error>;

    /// Sends data bytes to the display following a command.
    fn send_command_with_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), Self::Error>;

    /// Sends pixel data
    fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), Self::Error>;

    // fn read_data(&mut self, cmd: u8, buffer: &mut [u8], read_length: u8) -> Result<(), Self::Error>;
}

/// Trait for controlling the SH8601 hardware reset pin.
pub trait ResetInterface {
    /// The specific error type for this reset implementation.
    type Error;

    /// Performs the hardware reset sequence according to the SH8601 datasheet definition.
    /// This could be a GPIO port or an I2C expander-controlled pin.
    /// Implmentation should clear the reset line and wait for 20 ms then set the line high and wait for 150 ms.
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// SH8601 Command Set
pub mod commands {
    pub const NOP: u8 = 0x00;
    pub const SWRESET: u8 = 0x01;
    pub const RDDIDIF: u8 = 0x04; // Read Display Identification Information
    pub const RDDPM: u8 = 0x0A; // Read Display Power Mode
    pub const RDDMADCTL: u8 = 0x0B; // Read Display MADCTL
    pub const RDDCOLMOD: u8 = 0x0C; // Read Display Pixel Format
    pub const RDDSDR: u8 = 0x0F; // Read Display Self-Diagnostic Result
    pub const SLPIN: u8 = 0x10;
    pub const SLPOUT: u8 = 0x11;
    pub const PTLON: u8 = 0x12; // Partial Display Mode On
    pub const NORON: u8 = 0x13; // Normal Display Mode On
    pub const INVOFF: u8 = 0x20; // Display Inversion Off
    pub const INVON: u8 = 0x21; // Display Inversion On
    pub const DISPOFF: u8 = 0x28;
    pub const DISPON: u8 = 0x29;
    pub const CASET: u8 = 0x2A; // Column Address Set
    pub const PASET: u8 = 0x2B; // Page Address Set
    pub const RAMWR: u8 = 0x2C; // Memory Write Start
    pub const PTLAR: u8 = 0x30; // Partial Area Row Set
    pub const TEOFF: u8 = 0x34; // Tearing Effect Off
    pub const TEON: u8 = 0x35; // Tearing Effect On
    pub const MADCTL: u8 = 0x36; // Memory Data Access Control
    pub const IDMOFF: u8 = 0x38; // Idle Mode Off
    pub const IDMON: u8 = 0x39; // Idle Mode On
    pub const COLMOD: u8 = 0x3A; // Control Interface Pixel Format
    pub const RAMWRC: u8 = 0x3C; // Memory Write Continue
    pub const TESCAN: u8 = 0x44; // Set Tear Scan Line
    pub const WRDISBV: u8 = 0x51; // Write Display Brightness Value
    pub const RDDISBV: u8 = 0x52; // Read Display Brightness Value
    pub const WRCTRLD1: u8 = 0x53; // Write CTRL Display 1
    pub const RDCTRLD1: u8 = 0x54; // Read CTRL Display 1
}

/// Color modes supported by the SH8601 display controller.
pub enum ColorMode {
    /// 16-bit RGB565 format
    Rgb565,
    /// 24-bit RGB888 format
    Rgb888,
    /// 18-bit RGB666 format
    Rgb666,
    /// 8-bit 256 Gray
    Gray8,
}

impl ColorMode {
    /// Returns the number of bytes per pixel for the color format.
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            ColorMode::Rgb565 => 2,
            ColorMode::Rgb888 => 3,
            ColorMode::Rgb666 => 3,
            ColorMode::Gray8 => 1,
        }
    }
}

/// Computes the framebuffer size (in bytes) for a given display and color mode.
/// Recommended to use when defining generic constant framebuffer size (const `N`) when instantiating display controller driver with `new_static` and `new_heap`.
pub const fn framebuffer_size(display: DisplaySize, color: ColorMode) -> usize {
    (display.width as usize) * (display.height as usize) * color.bytes_per_pixel()
}

/// Frambuffer enum to hold either a static array or a boxed array
pub enum Framebuffer {
    Static(&'static mut [u8]),
    Heap(Box<[u8]>),
}

impl Framebuffer {
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
}

// Implement Framebuffer Deref to allow accessing the underlying array
impl core::ops::Deref for Framebuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
        }
    }
}

// Implement Frambuffer DerefMut for mutable access
impl core::ops::DerefMut for Framebuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
        }
    }
}

/// Main Driver for the SH8601 display controller.
///
/// Generic over the display interface (`IFACE`) and reset pin (`RST`).
pub struct Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    interface: IFACE,
    reset: RST,
    framebuffer: Framebuffer,
    config: DisplaySize,
}

impl<IFACE, RST> Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    /// Creates a new driver instance with static array and initializes the display.
    /// N is a constant representing the framebuffer size (number of pixels).
    /// N is calculated as `DisplaySize.width * DisplaySize.height * bytes_per_pixel`, where `bytes_per_pixel` is 2 for RGB565, 3 for RGB888, etc.
    /// You can use the `framebuffer_size` helper function to calculate N.
    pub fn new_static<DELAY, const N: usize>(
        interface: IFACE,
        reset: RST,
        color: ColorMode,
        config: DisplaySize,
        mut delay: DELAY,
        framebuffer: &'static mut [u8; N],
    ) -> Result<Self, DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        // Create a static array filled with zeros.

        let mut driver = Self {
            interface,
            reset,
            framebuffer: Framebuffer::Static(&mut framebuffer[..]),
            config,
        };
        driver.hard_reset()?;
        driver.initialize_display(&mut delay, color)?;
        Ok(driver)
    }

    /// Creates a new driver instance with a boxed array framebuffer.
    /// N is a constant representing the framebuffer size (number of pixels).
    /// N is calculated as `DisplaySize.width * DisplaySize.height * bytes_per_pixel`, where `bytes_per_pixel` is 2 for RGB565, 3 for RGB888, etc.
    pub fn new_heap<DELAY, const N: usize>(
        interface: IFACE,
        reset: RST,
        color: ColorMode,
        config: DisplaySize,
        mut delay: DELAY,
    ) -> Result<Self, DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        // Create a boxed array filled with zeros.
        let mut driver = Self {
            interface,
            reset,
            framebuffer: Framebuffer::Heap(Box::new([0u8; N])),
            config,
        };
        driver.hard_reset()?;
        driver.initialize_display(&mut delay, color)?;
        Ok(driver)
    }

    /// Performs a hardware reset using the provided `ResetPin` implementation,
    pub fn hard_reset(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.reset.reset().map_err(DriverError::ResetError)?;
        Ok(())
    }

    /// Sends the essential initialization command sequence to the display.
    pub fn initialize_display<DELAY>(
        &mut self,
        delay: &mut DELAY,
        color: ColorMode,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        self.send_command(commands::SWRESET)?;
        delay.delay_ms(10);
        self.send_command(commands::SLPOUT)?;
        delay.delay_ms(120);
        match color {
            ColorMode::Rgb565 => {
                // Set pixel format to RGB565
                self.send_command_with_data(commands::COLMOD, &[0x55])?;
            }
            ColorMode::Rgb888 => {
                // Set pixel format to RGB888
                self.send_command_with_data(commands::COLMOD, &[0x77])?;
            }
            ColorMode::Rgb666 => {
                // Set pixel format to RGB666
                self.send_command_with_data(commands::COLMOD, &[0x66])?;
            }
            ColorMode::Gray8 => {
                // Set pixel format to 8-bit grayscale
                self.send_command_with_data(commands::COLMOD, &[0x11])?;
            }
        }
        delay.delay_ms(5);
        self.send_command_with_data(commands::MADCTL, &[0x00])?;
        self.send_command_with_data(commands::TESCAN, &[0x01, 0xC5])?;
        self.send_command_with_data(commands::TEON, &[0x00])?;

        self.send_command(commands::DISPON)?;
        delay.delay_ms(120);

        self.send_command_with_data(commands::PTLAR, &[0x00, 0x80, 0x00, 0x02])?;
        delay.delay_ms(10);

        Ok(())
    }

    /// Send a command with no data
    fn send_command(&mut self, cmd: u8) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.interface
            .send_command(cmd)
            .map_err(DriverError::InterfaceError)
    }

    /// Helper to send a command with associated data parameters
    fn send_command_with_data(
        &mut self,
        cmd: u8,
        data: &[u8],
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.interface
            .send_command_with_data(cmd, data)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }

    /// Sleep Mode In (SLPIN)
    pub fn sleep_in<DELAY>(
        &mut self,
        delay: &mut DELAY,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        self.send_command(commands::SLPIN)?;
        delay.delay_ms(5);
        Ok(())
    }

    /// SH8601 Sleep Out (SLPOUT)
    pub fn sleep_out<DELAY>(
        &mut self,
        delay: &mut DELAY,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        self.send_command(commands::SLPOUT)?;
        delay.delay_ms(5);
        Ok(())
    }

    /// Turns the display panel off
    pub fn display_off(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command(commands::DISPOFF)
    }

    /// Turns the display panel on
    pub fn display_on(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command(commands::DISPON)
    }

    /// Sets the active drawing window on the display RAM.
    pub fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        if x_end == 0 || y_end == 0 {
            return Err(DriverError::InvalidConfiguration(
                "Window width/height cannot be zero",
            ));
        }
        if x_start >= self.config.width || y_start >= self.config.height {
            return Err(DriverError::InvalidConfiguration(
                "Window start coordinates out of bounds",
            ));
        }

        if x_end < x_start || y_end < y_start {
            return Err(DriverError::InvalidConfiguration(
                "Invalid window dimensions (end < start)",
            ));
        }

        // CASET (2Ah): Column Address Set
        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start >> 8) as u8,
                (x_start & 0xFF) as u8, // Start Column SC[15:0]
                (x_end >> 8) as u8,
                (x_end & 0xFF) as u8, // End Column EC[15:0]
            ],
        )?;

        // PASET (2Bh): Page Address Set
        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start >> 8) as u8,
                (y_start & 0xFF) as u8, // Start Page SP[15:0]
                (y_end >> 8) as u8,
                (y_end & 0xFF) as u8, // End Page EP[15:0]
            ],
        )?;
        Ok(())
    }

    /// Sets the Memory Data Access Control (MADCTL) register (controls orientation, color order).
    pub fn set_madctl(&mut self, value: u8) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command_with_data(commands::MADCTL, &[value])
    }

    /// Sets the display brightness (0x000 - 0x3FF).
    pub fn set_brightness(
        &mut self,
        value: u16,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        let brightness = value.min(0x3FF); // Clamp to 10 bits (0-1023)
        let val_lsb = (brightness & 0xFF) as u8;
        let val_msb = ((brightness >> 8) & 0x03) as u8;
        self.send_command_with_data(commands::WRDISBV, &[val_lsb, val_msb])
    }

    /// Writes the contents of the framebuffer to the display RAM.
    /// This is typically called after drawing operations are complete.
    pub fn flush(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        // Ensure the window covers the whole framebuffer before writing
        self.set_window(0, 0, self.config.width - 1, self.config.height - 1)?;
        // Send the pixel data via the interface's optimized method.
        // The send_pixels method itself should handle sending RAMWR (0x2C).
        self.interface
            .send_pixels(&self.framebuffer)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }

    pub fn partial_flush(
        &mut self,
        x_start: u16,
        x_end: u16,
        y_start: u16,
        y_end: u16,
        color: ColorMode,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.set_window(x_start, y_start, x_end, y_end)?;
        let bytes_per_pixel = color.bytes_per_pixel();
        let fb_width = self.config.width as usize * bytes_per_pixel;
        let width = (x_end - x_start + 1) as usize;
        let height = (y_end - y_start + 1) as usize;
        let mut pixel_data = alloc::vec::Vec::with_capacity(width * height * bytes_per_pixel);

        for y in 0..height {
            let offset = (y_start as usize + y) * fb_width + (x_start as usize * bytes_per_pixel);
            let row_end = offset + (width * bytes_per_pixel);
            if offset < self.framebuffer.len() && row_end <= self.framebuffer.len() {
                pixel_data.extend_from_slice(&self.framebuffer[offset..row_end]);
            } else {
                return Err(DriverError::InvalidConfiguration(
                    "Framebuffer slice out of bounds",
                ));
            }
        }

        self.interface
            .send_pixels(&pixel_data)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }
}
