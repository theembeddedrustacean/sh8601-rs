//! Driver implementation for Waveshare ESP32-S3 1.8" AMOLED
//! Uses QSPI interface and I2C-based GPIO expander or GPIO for reset.

use crate::{ControllerInterface, ResetInterface, commands};
use esp_hal::{
    Blocking,
    delay::Delay,
    spi::{
        Error as SpiError,
        master::{Address, Command, DataMode, SpiDmaBus},
    },
};

const CMD_RAMWR: u32 = 0x2C;
const CMD_RAMWRC: u32 = 0x3C;
const QSPI_PIXEL_OPCODE: u8 = 0x32;
const QSPI_CONTROL_OPCODE: u8 = 0x02;
pub const DMA_CHUNK_SIZE: usize = 16380;

/// QSPI implementation of ControllerInterface for SH8601
pub struct Ws143TouchAmoledDriver {
    pub qspi: SpiDmaBus<'static, Blocking>,
    pub use_co5300_init_cmds: bool,
}

impl Ws143TouchAmoledDriver {
    pub fn new(qspi: SpiDmaBus<'static, Blocking>) -> Self {
        Ws143TouchAmoledDriver {
            qspi,
            use_co5300_init_cmds: false,
        }
    }

    /// Seems like some waveshare 1.43 have a different lcd driver (co5300 instead of sh8601)
    /// which have a different init sequence but still work with this driver
    pub fn use_co5300_init_cmds(self) -> Self {
        Self {
            use_co5300_init_cmds: true,
            ..self
        }
    }
}

impl ControllerInterface for Ws143TouchAmoledDriver {
    type Error = SpiError;

    fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
        let address_value = (cmd as u32) << 8;

        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            &[],
        )?;
        Ok(())
    }

    fn send_command_with_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), Self::Error> {
        let address_value = (cmd as u32) << 8;

        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            data,
        )?;
        Ok(())
    }

    fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), Self::Error> {
        let ramwr_addr_val = (CMD_RAMWR as u32) << 8;
        let ramwrc_addr_val = (CMD_RAMWRC as u32) << 8;

        let mut chunks = pixels.chunks(DMA_CHUNK_SIZE).enumerate();

        while let Some((index, chunk)) = chunks.next() {
            if index == 0 {
                self.qspi.half_duplex_write(
                    DataMode::Quad,
                    Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                    Address::_24Bit(ramwr_addr_val, DataMode::Single),
                    0,
                    chunk,
                )?;
            } else {
                self.qspi.half_duplex_write(
                    DataMode::Quad,
                    Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                    Address::_24Bit(ramwrc_addr_val, DataMode::Single),
                    0,
                    chunk,
                )?;
            }
        }
        Ok(())
    }

    fn vendor_specific_init_commands(&self) -> &'static [(u8, &'static [u8], u32)] {
        if self.use_co5300_init_cmds {
            // co5300 init sequence
            &[
                (commands::SLPOUT, &[], 120),
                (commands::SPI_MODE, &[0x80], 0),
                (commands::WRCTRLD1, &[0x20], 1),
                (commands::HBM_WRDISBV1, &[0xFF], 1),
                (commands::WRDISBV, &[0x00], 1),
                (commands::SLPOUT, &[0x00], 10),
                (commands::WRDISBV, &[0xFF], 0),
            ]
        } else {
            // sh8601 init sequence
            &[
                (commands::SLPOUT, &[], 120),
                (commands::TESCAN, &[0x01, 0xD1], 0),
                (commands::TEON, &[0x00], 0),
                (commands::WRCTRLD1, &[0x20], 10),
                (commands::WRDISBV, &[0x00], 10),
                (commands::SLPOUT, &[0x00], 10),
                (commands::WRDISBV, &[0xFF], 0),
            ]
        }
    }
}

/// I2C-controlled GPIO Reset Pin
pub struct ResetDriver<IO> {
    output: IO,
}

impl<IO> ResetDriver<IO> {
    pub fn new(output: IO) -> Self {
        ResetDriver { output }
    }
}

impl<IO> ResetInterface for ResetDriver<IO>
where
    IO: embedded_hal::digital::OutputPin,
{
    type Error = IO::Error;

    fn reset(&mut self) -> Result<(), Self::Error> {
        let delay = Delay::new();
        self.output.set_low()?;
        delay.delay_millis(20);
        self.output.set_high()?;
        delay.delay_millis(150);
        Ok(())
    }
}
