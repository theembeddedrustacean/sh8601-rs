# SH8601 Driver Crate

An embedded-graphics compatible driver for the SH8601 AMOLED display controller IC.

This driver is not embedded-hal compatible but provides a generic interface for controlling the SH8601 display controller. Different displays can be supported by implementing the `ControllerInterface` and `ResetInterface` traits. This is because the SH8601 is used in different displays with various controller interfaces such as SPI or QSPI. Additionally, the reset pin is controlled via GPIO or I2C GPIO expander.

The driver currently incorporates support for the Waveshare 1.8" AMOLED display out of the box but can be extended to support other displays using the SH8601 controller.

## Usage

1. Implement the `ControllerInterface` trait for the controller driving interface (e.g., QSPI).
2. Implement the `ResetInterface` trait for the Reset pin.
3. Create a `Sh8601Driver` instance with the display interface and reset pin.
4. Use the driver to draw using `embedded-graphics`.

If you are going to use a heap-allocated framebuffer, you will need to ensure that an allocator is available in your environment. In some crates, this is done by enabling the `alloc` feature.

## Examples

See the `examples` directory for a usage example with the WaveShare 1.8" AMOLED Display.

The WaveShare 1.8" AMOLED Display controls the SH8601 via an ESP32-S3 over QSPI and uses an I2C GPIO expander for the reset pin. The example implementation uses a PSRAM heap-allocated framebuffer and DMA for efficient transfers.

The schematic is available here: Waveshare ESP32-S3-Touch-AMOLED-1.8

To run the example, with the WaveShare 1.8" AMOLED Display, clone the project and run following command from the project root:

```bash
cargo run --release --example ws_18in_amoled --features "waveshare_18_amoled"
```
