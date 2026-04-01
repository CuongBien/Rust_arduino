#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::Adc;

const GAS_LOW_THRESHOLD: u16 = 300;
const GAS_HIGH_THRESHOLD: u16 = 650;

const DEGREES_LOW: u16 = 30;
const DEGREES_MID: u16 = 60;
const DEGREES_HIGH: u16 = 120;

const STEPS_PER_REV: u32 = 2048;
const STEP_DELAY_MS: u32 = 5;
const PAUSE_BETWEEN_MS: u32 = 500;

const fn steps_for_degrees(deg: u16) -> u16 {
    ((deg as u32 * STEPS_PER_REV + 180) / 360) as u16
}

const STEPS_LOW: u16 = steps_for_degrees(DEGREES_LOW);
const STEPS_MID: u16 = steps_for_degrees(DEGREES_MID);
const STEPS_HIGH: u16 = steps_for_degrees(DEGREES_HIGH);

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

    // ULN2003 inputs: IN1=D8, IN2=D9, IN3=D10, IN4=D11.
    let mut in1 = pins.d8.into_output();
    let mut in2 = pins.d9.into_output();
    let mut in3 = pins.d10.into_output();
    let mut in4 = pins.d11.into_output();
    in1.set_low();
    in2.set_low();
    in3.set_low();
    in4.set_low();

    // Gas analog sensor on A0.
    let mut adc = Adc::new(dp.ADC, Default::default());
    let gas_pin = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "=== STEPPER GAS AUTO ===\r").ok();
    ufmt::uwriteln!(
        &mut serial,
        "LOW<{} MID<{} HIGH>={} (A0)\r",
        GAS_LOW_THRESHOLD,
        GAS_HIGH_THRESHOLD,
        GAS_HIGH_THRESHOLD
    )
    .ok();

    loop {
        let gas_value: u16 = adc.read_blocking(&gas_pin);

        if gas_value >= GAS_HIGH_THRESHOLD {
            // High gas: reverse 120 degrees.
            ufmt::uwriteln!(
                &mut serial,
                "gas={} -> NGHICH {}deg ({} steps)\r",
                gas_value,
                DEGREES_HIGH,
                STEPS_HIGH
            )
            .ok();
            for step in 0..STEPS_HIGH {
                let idx = (3 - (step % 4)) as usize;
                let pat = SEQ[idx];
                if pat[0] == 1 { in1.set_high() } else { in1.set_low() }
                if pat[1] == 1 { in2.set_high() } else { in2.set_low() }
                if pat[2] == 1 { in3.set_high() } else { in3.set_low() }
                if pat[3] == 1 { in4.set_high() } else { in4.set_low() }
                arduino_hal::delay_ms(STEP_DELAY_MS);
            }
        } else if gas_value >= GAS_LOW_THRESHOLD {
            // Mid gas: forward 60 degrees.
            ufmt::uwriteln!(
                &mut serial,
                "gas={} -> THUAN {}deg ({} steps)\r",
                gas_value,
                DEGREES_MID,
                STEPS_MID
            )
            .ok();
            for step in 0..STEPS_MID {
                let idx = (step % 4) as usize;
                let pat = SEQ[idx];
                if pat[0] == 1 { in1.set_high() } else { in1.set_low() }
                if pat[1] == 1 { in2.set_high() } else { in2.set_low() }
                if pat[2] == 1 { in3.set_high() } else { in3.set_low() }
                if pat[3] == 1 { in4.set_high() } else { in4.set_low() }
                arduino_hal::delay_ms(STEP_DELAY_MS);
            }
        } else {
            // Low gas: forward 30 degrees.
            ufmt::uwriteln!(
                &mut serial,
                "gas={} -> THUAN {}deg ({} steps)\r",
                gas_value,
                DEGREES_LOW,
                STEPS_LOW
            )
            .ok();
            for step in 0..STEPS_LOW {
                let idx = (step % 4) as usize;
                let pat = SEQ[idx];
                if pat[0] == 1 { in1.set_high() } else { in1.set_low() }
                if pat[1] == 1 { in2.set_high() } else { in2.set_low() }
                if pat[2] == 1 { in3.set_high() } else { in3.set_low() }
                if pat[3] == 1 { in4.set_high() } else { in4.set_low() }
                arduino_hal::delay_ms(STEP_DELAY_MS);
            }
        }

        // Turn off coils between moves to reduce heat.
        in1.set_low();
        in2.set_low();
        in3.set_low();
        in4.set_low();
        arduino_hal::delay_ms(PAUSE_BETWEEN_MS);
    }
}

