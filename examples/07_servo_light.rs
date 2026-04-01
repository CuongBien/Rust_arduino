#![no_std]
#![no_main]

use arduino_hal::Adc;
use panic_halt as _;

const LIGHT_THRESHOLD: u16 = 500;
const SERVO_PULSE_BRIGHT_US: u32 = 2000; // ~180 degree (tune per servo)
const SERVO_PULSE_DARK_US: u32 = 1000;   // ~0 degree (tune per servo)
const SERVO_PERIOD_US: u32 = 20_000;     // 50Hz
const SERVO_HOLD_CYCLES: u8 = 30;        // hold ~600ms

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let mut servo = pins.d9.into_output();

    let mut adc = Adc::new(dp.ADC, Default::default());
    let light_pin = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "=== SERVO + LIGHT ===\r").ok();
    ufmt::uwriteln!(
        &mut serial,
        "TH={} BRIGHT={}us DARK={}us\r",
        LIGHT_THRESHOLD,
        SERVO_PULSE_BRIGHT_US,
        SERVO_PULSE_DARK_US
    )
    .ok();

    loop {
        let light_value: u16 = adc.read_blocking(&light_pin);
        // In this wiring, dark gives higher ADC and bright gives lower ADC.
        let pulse_us: u32 = if light_value <= LIGHT_THRESHOLD {
            SERVO_PULSE_BRIGHT_US
        } else {
            SERVO_PULSE_DARK_US
        };

        if light_value <= LIGHT_THRESHOLD {
            ufmt::uwriteln!(&mut serial, "light={} -> SERVO BRIGHT\r", light_value).ok();
        } else {
            ufmt::uwriteln!(&mut serial, "light={} -> SERVO DARK\r", light_value).ok();
        }

        for _ in 0..SERVO_HOLD_CYCLES {
            servo.set_high();
            arduino_hal::delay_us(pulse_us);
            servo.set_low();
            arduino_hal::delay_us(SERVO_PERIOD_US - pulse_us);
        }
    }
}

