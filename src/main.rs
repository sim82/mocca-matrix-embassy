// //! This example test the RP Pico on board LED.
// //!
// //! It does not work with the RP Pico W board. See wifi_blinky.rs.

// #![no_std]
// #![no_main]

// use defmt::*;
// use embassy_executor::Spawner;
// use embassy_rp::gpio;
// use embassy_time::Timer;
// use gpio::{Level, Output};
// use {defmt_rtt as _, panic_probe as _};

// #[embassy_executor::main]
// async fn main(_spawner: Spawner) {
//     let p = embassy_rp::init(Default::default());
//     let mut led = Output::new(p.PIN_25, Level::Low);

//     loop {
//         info!("led on!");
//         led.set_high();
//         Timer::after_secs(1).await;

//         info!("led off!");
//         led.set_low();
//         Timer::after_secs(1).await;
//     }
// }

//! This example shows powerful PIO module in the RP2040 chip to communicate with WS2812 LED modules.
//! See (https://www.sparkfun.com/categories/tags/ws2812)

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_time::{Duration, Ticker, Timer};
use smart_leds::RGB8;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

/// Input a value 0 to 255 to get a color value
/// The colours are a transition r - g - b - back to r.
fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return (255 - wheel_pos * 3, 0, wheel_pos * 3).into();
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return (0, wheel_pos * 3, 255 - wheel_pos * 3).into();
    }
    wheel_pos -= 170;
    (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
}

#[embassy_executor::task]
async fn blink_task(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after_millis(100).await;
        led.set_low();
        Timer::after_millis(100).await;
    }
}
const NUM_LEDS: usize = 8;
#[embassy_executor::task]
async fn rgb_task(mut ws2812: PioWs2812<'static, PIO0, 0, NUM_LEDS>) {
    // This is the number of leds in the string. Helpfully, the sparkfun thing plus and adafruit
    // feather boards for the 2040 both have one built in.
    let mut data = [RGB8::default(); NUM_LEDS];
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        for j in 0..(256 * 5) {
            // debug!("New Colors:");
            for i in 0..NUM_LEDS {
                data[i] = wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8);
                // debug!("R: {} G: {} B: {}", data[i].r, data[i].g, data[i].b);
            }
            ws2812.write(&data).await;

            ticker.next().await;
        }
    }
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Start");
    let p = embassy_rp::init(Default::default());

    let led = Output::new(p.PIN_25, Level::Low);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    // Common neopixel pins:
    // Thing plus: 8
    // Adafruit Feather: 16;  Adafruit Feather+RFM95: 4
    let program = PioWs2812Program::new(&mut common);
    let ws2812 = PioWs2812::new(&mut common, sm0, p.DMA_CH0, p.PIN_16, &program);

    // Loop forever making RGB values and pushing them out to the WS2812.
    unwrap!(spawner.spawn(blink_task(led)));
    unwrap!(spawner.spawn(rgb_task(ws2812)));
}
