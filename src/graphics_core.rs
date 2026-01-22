use crate::{ControllerInterface, DrawTarget, ResetInterface, Sh8601Driver};
use embedded_graphics_core::{pixelcolor::Rgb888, prelude::*};

impl<IFACE, RST> DrawTarget for Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    type Color = Rgb888;
    // Drawing to the framebuffer in memory is infallible.
    // Errors happen during flush with SPI comms.
    type Error = core::convert::Infallible;

    /// Draws a single pixel to the internal framebuffer.
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < self.config.width as i32
                && coord.y >= 0
                && coord.y < self.config.height as i32
            {
                let x = coord.x as u32;
                let y = coord.y as u32;
                let index = ((y * self.config.width as u32 + x) * 3) as usize;

                if index + 2 < self.framebuffer.len() {
                    let c = color.into_storage();
                    let r = (c >> 16) as u8; // 8-bit Red
                    let g = (c >> 8) as u8; // 8-bit Green
                    let b = c as u8; // 8-bit Blue

                    self.framebuffer[index] = r;
                    self.framebuffer[index + 1] = g;
                    self.framebuffer[index + 2] = b;
                }
            }
        }
        Ok(())
    }

    /// Optimized clear - fill entire framebuffer
    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let c = color.into_storage();
        let r = (c >> 16) as u8;
        let g = (c >> 8) as u8;
        let b = c as u8;

        // Fast path for uniform colors (black, white, grayscale)
        if r == g && r == b {
            self.framebuffer.fill(r);

            return Ok(());
        }

        for chunk in self.framebuffer.chunks_exact_mut(3) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
        }

        Ok(())
    }
}

// =========== embedded-graphics OriginDimensions Implementation ===========

impl<IFACE, RST> OriginDimensions for Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    fn size(&self) -> Size {
        Size::new((self.config.width) as u32, (self.config.height) as u32)
    }
}
