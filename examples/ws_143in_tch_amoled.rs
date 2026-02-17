#![no_std]
#![no_main]

use sh8601_rs::{
    ColorMode, DMA_CHUNK_SIZE, DisplaySize, ResetDriver, Sh8601Driver, Ws143TouchAmoledDriver,
    framebuffer_size,
};

use embedded_graphics::{
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_6X10, FONT_10X20},
    },
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Alignment, LineHeight, Text, TextStyleBuilder},
};

extern crate alloc;
use esp_alloc as _;
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::{
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use esp_println::println;

esp_app_desc!();

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let delay = Delay::new();

    // --- DMA Buffers for SPI ---
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(DMA_CHUNK_SIZE);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    // SPI Configuration for Waveshare ESP32-S3 1.8inch AMOLED Touch Display
    // Hardware is configured for QSPI. Pinout obtained from the schematic.
    // Schematic:
    // https://files.waveshare.com/wiki/ESP32-S3-Touch-AMOLED-1.8/ESP32-S3-Touch-AMOLED-1.8.pdf
    // Using DMA for more efficient SPI communication.
    let lcd_spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40_u32))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sio0(peripherals.GPIO11)
    .with_sio1(peripherals.GPIO12)
    .with_sio2(peripherals.GPIO13)
    .with_sio3(peripherals.GPIO14)
    .with_cs(peripherals.GPIO9)
    .with_sck(peripherals.GPIO10)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf);

    // GPIO Configuration for Waveshare ESP32-S3 1.43inch AMOLED Touch Display
    // Display uses an GPIO to control the OLED_RESET and OLED_EN lines.
    // Pinout:
    // OLED_RESET -> GPIO21
    // OLED_EN -> GPIO42
    // Schematic:
    // https://files.waveshare.com/wiki/ESP32-S3-Touch-AMOLED-1.43/ESP32-S3-Touch-AMOLED-1.43-Schematic.pdf
    let oled_reset_pin = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());
    let mut oled_en_pin = Output::new(peripherals.GPIO42, Level::Low, OutputConfig::default());

    // Initialize I2C GPIO Reset Pin for the WaveShare 1.8" AMOLED display
    let reset = ResetDriver::new(oled_reset_pin);

    // Initialize display driver for the Waveshare 1.8" AMOLED display
    let ws_driver = Ws143TouchAmoledDriver::new(lcd_spi);

    // Set up the display size
    const DISPLAY_SIZE: DisplaySize = DisplaySize::new(466, 466);

    // Calculate framebuffer size based on the display size and color mode
    const FB_SIZE: usize = framebuffer_size(DISPLAY_SIZE, ColorMode::Rgb888);

    // Enable the display
    oled_en_pin.set_high();
    delay.delay_millis(100);

    // Instantiate and Initialize Display
    println!("Initializing SH8601 Display...");
    let display_res = Sh8601Driver::new_heap::<_, FB_SIZE>(
        ws_driver,
        reset,
        ColorMode::Rgb888,
        DISPLAY_SIZE,
        delay,
    );
    let mut display = match display_res {
        Ok(d) => {
            println!("Display initialized successfully.");
            d
        }
        Err(e) => {
            println!("Error initializing display: {:?}", e);
            loop {}
        }
    };

    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb888::WHITE);

    let text_style = TextStyleBuilder::new()
        .line_height(LineHeight::Pixels(300))
        .alignment(Alignment::Center)
        .build();

    let text = "Hello, Waveshare 1.8\" AMOLED Display!";

    // Triangle::new(Point::new(20, 50), Point::new(50, 5), Point::new(80, 50))
    //     .into_styled(
    //         PrimitiveStyleBuilder::new()
    //             .stroke_color(Rgb888::CSS_GOLD)
    //             .stroke_width(5)
    //             .fill_color(Rgb888::BLUE)
    //             .build(),
    //     )
    //     .draw(&mut display)
    //     .unwrap();

    // let style = PrimitiveStyleBuilder::new()
    //     .stroke_color(Rgb565::RED)
    //     .stroke_width(3)
    //     .fill_color(Rgb565::GREEN)
    //     .build();

    // Rectangle::new(Point::new(30, 30), Size::new(150, 150))
    //     .into_styled(style)
    //     .draw(&mut display)
    //     .unwrap();

    // Circle::new(Point::new(50, 100), 5)
    //     .into_styled(PrimitiveStyle::with_fill(Rgb888::RED))
    //     .draw(&mut display)
    //     .unwrap();

    for col in (0..DISPLAY_SIZE.width as i32).step_by(10) {
        Text::with_text_style(text, Point::new(col, 100), character_style, text_style)
            .draw(&mut display)
            .unwrap();
        if let Err(e) = display.flush() {
            println!("Error flushing display: {:?}", e);
        }
        delay.delay_millis(500);
        display.clear(Rgb888::BLACK).unwrap();
    }

    loop {
        delay.delay_millis(500);
    }
}
