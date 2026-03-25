#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer0Pwm};
use arduino_hal::port::{mode, Pin, PinOps};

const DUTY_FAST: u8 = 255;
const M3_A_BIT: u8 = 5;
const M3_B_BIT: u8 = 7;

fn read_dht11<PIN>(
    pin: Pin<mode::Input<mode::Floating>, PIN>,
) -> (Option<(u8, u8)>, Pin<mode::Input<mode::Floating>, PIN>)
where
    PIN: PinOps,
{
    let mut out = pin.into_output();
    out.set_low();
    arduino_hal::delay_ms(18);
    out.set_high();
    arduino_hal::delay_us(30);
    let pin_in = out.into_floating_input();

    fn wait_low<P: PinOps>(p: &Pin<mode::Input<mode::Floating>, P>, lim: u16) -> bool {
        let mut t = 0u16;
        while p.is_low()  { if t >= lim { return false; } t += 1; arduino_hal::delay_us(1); }
        true
    }
    fn wait_high<P: PinOps>(p: &Pin<mode::Input<mode::Floating>, P>, lim: u16) -> bool {
        let mut t = 0u16;
        while p.is_high() { if t >= lim { return false; } t += 1; arduino_hal::delay_us(1); }
        true
    }

    if !wait_high(&pin_in, 10_000) { return (None, pin_in); }
    if !wait_low(&pin_in,  10_000) { return (None, pin_in); }
    if !wait_high(&pin_in, 10_000) { return (None, pin_in); }

    let mut data = [0u8; 5];
    for byte in data.iter_mut() {
        let mut b = 0u8;
        for bit in (0..8).rev() {
            if !wait_low(&pin_in,  10_000) { return (None, pin_in); }
            arduino_hal::delay_us(40);
            if pin_in.is_high() { b |= 1 << bit; }
            if !wait_high(&pin_in, 10_000) { return (None, pin_in); }
        }
        *byte = b;
    }

    let crc = data[0].wrapping_add(data[1]).wrapping_add(data[2]).wrapping_add(data[3]);
    if crc != data[4] { return (None, pin_in); }

    (Some((data[2], data[0])), pin_in)
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // HW-130 (L293D shield) control lines:
    // - M3 PWM: D5
    // - 74HC595: CLK=D4, EN=D7, DATA=D8, LATCH=D12
    let mut dir_clk = pins.d4.into_output();
    let mut dir_en = pins.d7.into_output();
    let mut dir_ser = pins.d8.into_output();
    let mut dir_latch = pins.d12.into_output();

    // 74HC595 output-enable is active-low on this shield.
    dir_en.set_low();

    let timer0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut pwm_pin = pins.d5.into_output().into_pwm(&timer0);
    pwm_pin.enable();

    let mut dht_pin = pins.a0.into_floating_input();
    let mut dht_errors: u8 = 0;
    let mut latch_state: u8 = 0;

    ufmt::uwriteln!(&mut serial, "=== KHOI DONG ===\r").ok();

    loop {
        arduino_hal::delay_ms(2_000);
        // Fail-safe: default M3 OFF each cycle.
        latch_state &= !(1 << M3_A_BIT);
        latch_state &= !(1 << M3_B_BIT);

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
        pwm_pin.set_duty(0);

        let (res, pin_back) = read_dht11(dht_pin);
        dht_pin = pin_back;

        match res {
            Some((temp, hum)) => {
                dht_errors = 0;
                ufmt::uwriteln!(&mut serial, "T={}C H={}%\r", temp, hum).ok();

                if temp >= 35 {
                    // M3 forward: A=1, B=0
                    latch_state |= 1 << M3_A_BIT;
                    latch_state &= !(1 << M3_B_BIT);

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

                    pwm_pin.set_duty(DUTY_FAST);
                    ufmt::uwriteln!(&mut serial, "M3 ON FAST duty={}\r", DUTY_FAST).ok();
                } else {
                    latch_state &= !(1 << M3_A_BIT);
                    latch_state &= !(1 << M3_B_BIT);

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

                    pwm_pin.set_duty(0);
                    ufmt::uwriteln!(&mut serial, "M3 OFF (<35C)\r").ok();
                }
            }

            None => {
                dht_errors = dht_errors.saturating_add(1);
                ufmt::uwriteln!(&mut serial, "DHT11 loi! lan {}\r", dht_errors).ok();
                ufmt::uwriteln!(&mut serial, "DUNG KHAN CAP\r").ok();
            }
        }
    }
}