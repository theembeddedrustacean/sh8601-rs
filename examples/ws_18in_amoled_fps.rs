#![no_std]
#![no_main]

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use sh8601_rs::{
    ColorMode, DMA_CHUNK_SIZE, DisplaySize, ResetDriver, Sh8601Driver, Ws18AmoledDriver,
    framebuffer_size,
};

extern crate alloc;
use esp_alloc as _;
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::{
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::{Duration, Instant, Rate},
};
use esp_println::println;

esp_app_desc!();

const DISPLAY_SIZE: DisplaySize = DisplaySize::new(368, 448);
const FB_SIZE: usize = framebuffer_size(DISPLAY_SIZE, ColorMode::Rgb888);

const FPS_SAMPLE_WINDOW: Duration = Duration::from_secs(1);
const BAR_COUNT: usize = 8;
const BAR_WIDTH: u32 = 40;
const BAR_HEIGHT: u32 = 40;
const BAR_SPACING: i32 = 12;
const HORIZONTAL_SPEED_PX: i32 = 7;

fn wheel(step: u8) -> Rgb888 {
    let phase = (step / 85) % 3;
    let offset = (step % 85) * 3;

    match phase {
        0 => Rgb888::new(255u8.saturating_sub(offset), offset, 0),
        1 => Rgb888::new(0, 255u8.saturating_sub(offset), offset),
        _ => Rgb888::new(offset, 0, 255u8.saturating_sub(offset)),
    }
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let delay = Delay::new();

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(DMA_CHUNK_SIZE);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let lcd_spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40_u32))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sio0(peripherals.GPIO4)
    .with_sio1(peripherals.GPIO5)
    .with_sio2(peripherals.GPIO6)
    .with_sio3(peripherals.GPIO7)
    .with_cs(peripherals.GPIO12)
    .with_sck(peripherals.GPIO11)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf);

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO15)
    .with_scl(peripherals.GPIO14);

    let reset = ResetDriver::new(i2c);
    let ws_driver = Ws18AmoledDriver::new(lcd_spi);

    println!("Initializing SH8601 display for FPS benchmark...");
    let display_res = Sh8601Driver::new_heap::<_, FB_SIZE>(
        ws_driver,
        reset,
        ColorMode::Rgb888,
        DISPLAY_SIZE,
        delay,
    );

    let mut display = match display_res {
        Ok(d) => d,
        Err(e) => {
            println!("Display init failed: {:?}", e);
            loop {}
        }
    };

    println!("Running benchmark... printing FPS every second");

    let mut sample_start = Instant::now();
    let mut frame_count: u32 = 0;
    let mut total_flush_time_us: u64 = 0;
    let mut phase: u8 = 0;
    let mut x_offset: i32 = 0;

    loop {
        display.clear(Rgb888::new(8, 8, 10)).unwrap();

        for i in 0..BAR_COUNT {
            let bar_color = wheel(phase.wrapping_add((i as u8).wrapping_mul(31)));
            let y_step = DISPLAY_SIZE.height as i32 / BAR_COUNT as i32;
            let y = (i as i32 * y_step) + ((y_step - BAR_HEIGHT as i32) / 2);

            let span = DISPLAY_SIZE.width as i32 + BAR_WIDTH as i32;
            let x = ((x_offset + i as i32 * (BAR_WIDTH as i32 + BAR_SPACING)) % span)
                - BAR_WIDTH as i32;

            Rectangle::new(Point::new(x, y), Size::new(BAR_WIDTH, BAR_HEIGHT))
                .into_styled(PrimitiveStyle::with_fill(bar_color))
                .draw(&mut display)
                .unwrap();
        }

        let flush_start = Instant::now();
        if let Err(e) = display.flush() {
            println!("Flush failed: {:?}", e);
            loop {}
        }
        total_flush_time_us = total_flush_time_us.saturating_add(flush_start.elapsed().as_micros());

        frame_count = frame_count.saturating_add(1);
        phase = phase.wrapping_add(3);

        let travel = DISPLAY_SIZE.width as i32 + BAR_WIDTH as i32;
        x_offset += HORIZONTAL_SPEED_PX;
        if x_offset >= travel {
            x_offset -= travel;
        }

        let elapsed = sample_start.elapsed();
        if elapsed >= FPS_SAMPLE_WINDOW {
            let elapsed_ms = elapsed.as_millis().max(1);
            let fps_x100 = (u64::from(frame_count) * 100_000) / elapsed_ms;
            let avg_flush_us = total_flush_time_us / u64::from(frame_count.max(1));

            println!(
                "fps={}.{:02} frames={} sample={}ms avg_flush={}us",
                fps_x100 / 100,
                fps_x100 % 100,
                frame_count,
                elapsed_ms,
                avg_flush_us,
            );

            sample_start = Instant::now();
            frame_count = 0;
            total_flush_time_us = 0;
        }
    }
}
