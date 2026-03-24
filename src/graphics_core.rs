use crate::{ControllerInterface, DrawTarget, ResetInterface, Sh8601Color, Sh8601Driver};
use embedded_graphics_core::prelude::*;
use embedded_graphics_core::primitives::Rectangle;

impl<IFACE, RST, COLOR> DrawTarget for Sh8601Driver<IFACE, RST, COLOR>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
    COLOR: Sh8601Color,
{
    type Color = COLOR;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let w = self.config.width as i32;
        let h = self.config.height as i32;
        let bpp = COLOR::BYTES_PER_PIXEL;
        let stride = self.config.width as usize * bpp;

        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0 && coord.x < w && coord.y >= 0 && coord.y < h {
                let offset = coord.y as usize * stride + coord.x as usize * bpp;
                color.encode(&mut self.framebuffer[offset..offset + bpp]);
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let disp_w = self.config.width as i32;
        let disp_h = self.config.height as i32;
        let bpp = COLOR::BYTES_PER_PIXEL;
        let stride = self.config.width as usize * bpp;

        let x0 = area.top_left.x.max(0);
        let y0 = area.top_left.y.max(0);
        let x1 = (area.top_left.x + area.size.width as i32).min(disp_w);
        let y1 = (area.top_left.y + area.size.height as i32).min(disp_h);

        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let area_w = area.size.width as usize;
        let clipped_w = (x1 - x0) as usize;
        let skip_left = (x0 - area.top_left.x) as usize;
        let skip_right = (area_w as i32 - (x1 - area.top_left.x)) as usize;
        let skip_top = ((y0 - area.top_left.y) as usize) * area_w;

        let fb = self.framebuffer.as_mut_slice();
        let mut iter = colors.into_iter();

        for _ in 0..skip_top {
            iter.next();
        }

        for y in y0..y1 {
            let row_start = y as usize * stride + x0 as usize * bpp;

            for _ in 0..skip_left {
                iter.next();
            }

            let row = &mut fb[row_start..row_start + clipped_w * bpp];
            for chunk in row.chunks_exact_mut(bpp) {
                if let Some(color) = iter.next() {
                    color.encode(chunk);
                } else {
                    return Ok(());
                }
            }

            for _ in 0..skip_right {
                iter.next();
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let disp_w = self.config.width as i32;
        let disp_h = self.config.height as i32;
        let bpp = COLOR::BYTES_PER_PIXEL;
        let stride = self.config.width as usize * bpp;

        let x0 = area.top_left.x.max(0) as usize;
        let y0 = area.top_left.y.max(0) as usize;
        let x1 = (area.top_left.x + area.size.width as i32).min(disp_w) as usize;
        let y1 = (area.top_left.y + area.size.height as i32).min(disp_h) as usize;

        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let row_bytes = (x1 - x0) * bpp;
        let fb = self.framebuffer.as_mut_slice();

        for y in y0..y1 {
            let start = y * stride + x0 * bpp;
            COLOR::fill_row(color, &mut fb[start..start + row_bytes]);
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        COLOR::fill_buf(color, &mut self.framebuffer);
        Ok(())
    }
}

impl<IFACE, RST, COLOR> OriginDimensions for Sh8601Driver<IFACE, RST, COLOR>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
    COLOR: Sh8601Color,
{
    fn size(&self) -> Size {
        Size::new(self.config.width as u32, self.config.height as u32)
    }
}
