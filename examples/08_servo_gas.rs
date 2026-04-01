#![no_std]
#![no_main]

use arduino_hal::Adc;
use panic_halt as _;

const GAS_THRESHOLD: u16 = 100;
const SERVO_PULSE_GAS_HIGH_US: u32 = 2000; // Position when gas is high
const SERVO_PULSE_GAS_LOW_US: u32 = 1000;  // Position when gas is low
const SERVO_PERIOD_US: u32 = 20_000;       // 50Hz
const SERVO_HOLD_CYCLES: u8 = 30;          // hold ~600ms

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let mut servo = pins.d9.into_output();

    let mut adc = Adc::new(dp.ADC, Default::default());
    let gas_pin = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "=== SERVO + GAS ===\r").ok();
    ufmt::uwriteln!(
        &mut serial,
        "TH={} HIGH={}us LOW={}us\r",
        GAS_THRESHOLD,
        SERVO_PULSE_GAS_HIGH_US,
        SERVO_PULSE_GAS_LOW_US
    )
    .ok();

    loop {
        let gas_value: u16 = adc.read_blocking(&gas_pin);
        let pulse_us: u32 = if gas_value >= GAS_THRESHOLD {
            SERVO_PULSE_GAS_HIGH_US
        } else {
            SERVO_PULSE_GAS_LOW_US
        };

        if gas_value >= GAS_THRESHOLD {
            ufmt::uwriteln!(&mut serial, "gas={} -> SERVO GAS_HIGH\r", gas_value).ok();
        } else {
            ufmt::uwriteln!(&mut serial, "gas={} -> SERVO GAS_LOW\r", gas_value).ok();
        }

        for _ in 0..SERVO_HOLD_CYCLES {
            servo.set_high();
            arduino_hal::delay_us(pulse_us);
            servo.set_low();
            arduino_hal::delay_us(SERVO_PERIOD_US - pulse_us);
        }
    }
}

