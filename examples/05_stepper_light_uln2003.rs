#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::Adc;

const LIGHT_THRESHOLD: u16 = 500;
const DEGREES_BRIGHT: u16 = 60;
const DEGREES_DARK: u16 = 90;

const STEPS_PER_REV: u32 = 2048;
const STEP_DELAY_MS: u32 = 5;
const PAUSE_BETWEEN_MS: u32 = 1000;

const fn steps_for_degrees(deg: u16) -> u16 {
    ((deg as u32 * STEPS_PER_REV + 180) / 360) as u16
}

const STEPS_BRIGHT: u16 = steps_for_degrees(DEGREES_BRIGHT); // 341
const STEPS_DARK: u16 = steps_for_degrees(DEGREES_DARK);     // 512

const SEQ: [[u8; 4]; 4] = [
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
];

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut in1 = pins.d8.into_output();
    let mut in2 = pins.d9.into_output();
    let mut in3 = pins.d10.into_output();
    let mut in4 = pins.d11.into_output();

    in1.set_low(); in2.set_low(); in3.set_low(); in4.set_low();

    let mut adc = Adc::new(dp.ADC, Default::default());
    let light_pin = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "=== STEPPER LIGHT ===\r").ok();
    ufmt::uwriteln!(&mut serial, "BRIGHT={}steps DARK={}steps\r", STEPS_BRIGHT, STEPS_DARK).ok();

    loop {
        let light_value: u16 = adc.read_blocking(&light_pin);

        if light_value >= LIGHT_THRESHOLD {
            ufmt::uwriteln!(&mut serial, "LIGHT={} -> THUAN 60deg ({} steps)\r", light_value, STEPS_BRIGHT).ok();
            for step in 0..STEPS_BRIGHT {
                let idx = (step % 4) as usize;
                let pat = SEQ[idx];
                if pat[0] == 1 { in1.set_high() } else { in1.set_low() }
                if pat[1] == 1 { in2.set_high() } else { in2.set_low() }
                if pat[2] == 1 { in3.set_high() } else { in3.set_low() }
                if pat[3] == 1 { in4.set_high() } else { in4.set_low() }
                arduino_hal::delay_ms(STEP_DELAY_MS);
            }
        } else {
            ufmt::uwriteln!(&mut serial, "LIGHT={} -> NGHICH 90deg ({} steps)\r", light_value, STEPS_DARK).ok();
            for step in 0..STEPS_DARK {
                let idx = (3 - (step % 4)) as usize;
                let pat = SEQ[idx];
                if pat[0] == 1 { in1.set_high() } else { in1.set_low() }
                if pat[1] == 1 { in2.set_high() } else { in2.set_low() }
                if pat[2] == 1 { in3.set_high() } else { in3.set_low() }
                if pat[3] == 1 { in4.set_high() } else { in4.set_low() }
                arduino_hal::delay_ms(STEP_DELAY_MS);
            }
        }

        in1.set_low(); in2.set_low(); in3.set_low(); in4.set_low();
        arduino_hal::delay_ms(PAUSE_BETWEEN_MS);
    }
}