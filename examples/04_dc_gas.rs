#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::port::{D12, D4, D8};
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer0Pwm};

/// ADC raw value threshold (0..1023). Tune with serial log `gas=...`.
const GAS_THRESHOLD: u16 = 80;
const DUTY_RUN: u8 = 255;

const M3_A_BIT: u8 = 5;
const M3_B_BIT: u8 = 7;

fn m3_shift_out(
    dir_clk: &mut arduino_hal::port::Pin<arduino_hal::port::mode::Output, D4>,
    dir_ser: &mut arduino_hal::port::Pin<arduino_hal::port::mode::Output, D8>,
    dir_latch: &mut arduino_hal::port::Pin<arduino_hal::port::mode::Output, D12>,
    latch_state: u8,
) {
    dir_latch.set_low();
    for i in (0..8).rev() {
        dir_clk.set_low();
        if ((latch_state >> i) & 1) == 1 {
            dir_ser.set_high();
        } else {
            dir_ser.set_low();
        }
        dir_clk.set_high();
    }
    dir_latch.set_high();
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // HW-130 (L293D shield) M3: PWM=D5, 74HC595 CLK=D4, EN=D7, DATA=D8, LATCH=D12
    let mut dir_clk = pins.d4.into_output();
    let mut dir_en = pins.d7.into_output();
    let mut dir_ser = pins.d8.into_output();
    let mut dir_latch = pins.d12.into_output();

    dir_en.set_low();

    let timer0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut pwm_pin = pins.d5.into_output().into_pwm(&timer0);
    pwm_pin.enable();

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let mut gas_pin = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "=== GAS M3 THUAN/NHICH ===\r").ok();
    ufmt::uwriteln!(
        &mut serial,
        "gas>= {} -> THUAN | gas< {} -> NHICH | DUTY={}\r",
        GAS_THRESHOLD,
        GAS_THRESHOLD,
        DUTY_RUN
    )
    .ok();

    loop {
        arduino_hal::delay_ms(1_000);

        let gas_value: u16 = adc.read_blocking(&mut gas_pin);

        // M3 forward: A=1 B=0 | reverse: A=0 B=1 (AFMotor / HW-130 convention)
        let mut latch_state: u8 = 0;
        if gas_value >= GAS_THRESHOLD {
            latch_state |= 1 << M3_A_BIT;
            latch_state &= !(1 << M3_B_BIT);
        } else {
            latch_state &= !(1 << M3_A_BIT);
            latch_state |= 1 << M3_B_BIT;
        }

        m3_shift_out(&mut dir_clk, &mut dir_ser, &mut dir_latch, latch_state);
        pwm_pin.set_duty(DUTY_RUN);

        if gas_value >= GAS_THRESHOLD {
            ufmt::uwriteln!(&mut serial, "gas={} THUAN\r", gas_value).ok();
        } else {
            ufmt::uwriteln!(&mut serial, "gas={} NHICH\r", gas_value).ok();
        }
    }
}
