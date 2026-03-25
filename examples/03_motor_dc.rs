#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Timer2Pwm, Prescaler};
use arduino_hal::port::{mode, Pin, PinOps};

const TEMP_HIGH: u8 = 30;
const TEMP_LOW: u8 = 25;
const HUM_HIGH: u8 = 75;

const SPEED_FAST: u8 = 255;
const SPEED_MID: u8 = 180;
const SPEED_SLOW: u8 = 100;
const SPEED_STOP: u8 = 0;

fn motor_decision(temp: u8, hum: u8) -> (u8, bool, bool) {
    match (temp >= TEMP_HIGH, temp >= TEMP_LOW, hum >= HUM_HIGH) {
        (true, _, _) => (SPEED_FAST, true, true),
        (false, true, true) => (SPEED_MID, true, true),
        (false, true, false) => (SPEED_SLOW, true, true),
        (false, false, true) => (SPEED_SLOW, false, true),
        (false, false, false) => (SPEED_STOP, true, false),
    }
}

/// Đọc DHT11 qua 1 chân GPIO (dạng 1-wire).
/// Trả về (temp_c, humidity_percent).
///
/// Wiring gợi ý:
/// - DHT11 DATA -> A0
/// - Có điện trở kéo lên (pull-up) ngoài (thường 10k) từ DATA lên 5V.
fn read_dht11<PIN>(
    pin: Pin<mode::Input<mode::Floating>, PIN>,
) -> (Option<(u8, u8)>, Pin<mode::Input<mode::Floating>, PIN>)
where
    PIN: PinOps,
{
    // Start signal: kéo LOW >= 18ms
    let mut out = pin.into_output();
    out.set_low();
    arduino_hal::delay_ms(18);
    out.set_high();
    arduino_hal::delay_us(30);

    let pin_in = out.into_floating_input();

    fn wait_while_low<PIN2>(pin: &Pin<mode::Input<mode::Floating>, PIN2>, limit_us: u16) -> bool
    where
        PIN2: PinOps,
    {
        let mut t: u16 = 0;
        while pin.is_low() {
            if t >= limit_us {
                return false;
            }
            t += 1;
            arduino_hal::delay_us(1);
        }
        true
    }

    fn wait_while_high<PIN2>(pin: &Pin<mode::Input<mode::Floating>, PIN2>, limit_us: u16) -> bool
    where
        PIN2: PinOps,
    {
        let mut t: u16 = 0;
        while pin.is_high() {
            if t >= limit_us {
                return false;
            }
            t += 1;
            arduino_hal::delay_us(1);
        }
        true
    }

    // DHT11 response: LOW ~80us, HIGH ~80us, then data starts
    if !wait_while_high(&pin_in, 10_000) {
        return (None, pin_in);
    }
    if !wait_while_low(&pin_in, 10_000) {
        return (None, pin_in);
    }
    if !wait_while_high(&pin_in, 10_000) {
        return (None, pin_in);
    }

    let mut data: [u8; 5] = [0; 5];
    for byte in data.iter_mut() {
        let mut b: u8 = 0;
        for bit in (0..8).rev() {
            // mỗi bit: LOW ~50us
            if !wait_while_low(&pin_in, 10_000) {
                return (None, pin_in);
            }

            // HIGH: 26-28us (0) hoặc ~70us (1)
            arduino_hal::delay_us(40);
            if pin_in.is_high() {
                b |= 1 << bit;
            }

            if !wait_while_high(&pin_in, 10_000) {
                return (None, pin_in);
            }
        }
        *byte = b;
    }

    let crc = data[0]
        .wrapping_add(data[1])
        .wrapping_add(data[2])
        .wrapping_add(data[3]);
    if crc != data[4] {
        return (None, pin_in);
    }

    let humidity: u8 = data[0];
    let temp_c: u8 = data[2];
    (Some((temp_c, humidity)), pin_in)
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Serial console. Nếu `Ravedude.toml` của bạn đang để 57600 thì giữ đúng 57600.
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // DC motor (1 kênh): D4/D7 hướng, D11 PWM tốc độ (giữ đúng skeleton ban đầu của bạn)
    let mut dir1 = pins.d4.into_output();
    let mut dir2 = pins.d7.into_output();
    let timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut pwm_pin = pins.d11.into_output().into_pwm(&timer2);
    pwm_pin.enable();

    // DHT11 DATA -> A0
    let mut dht_pin = pins.a0.into_floating_input();

    ufmt::uwriteln!(&mut serial, "DHT11(A0) + DC motor ready\r").ok();

    loop {
        arduino_hal::delay_ms(2_000);

        let (res, pin_back) = read_dht11(dht_pin);
        dht_pin = pin_back;

        match res {
            Some((temp_c, hum)) => {
                ufmt::uwriteln!(&mut serial, "T={}C H={}%\r", temp_c, hum).ok();

                let (duty, forward, run) = motor_decision(temp_c, hum);
                if run {
                    if forward {
                        dir1.set_high();
                        dir2.set_low();
                    } else {
                        dir1.set_low();
                        dir2.set_high();
                    }
                    pwm_pin.set_duty(duty);
                    ufmt::uwriteln!(
                        &mut serial,
                        "MOTOR duty={} fwd={}\r",
                        duty,
                        forward as u8
                    )
                    .ok();
                } else {
                    dir1.set_low();
                    dir2.set_low();
                    pwm_pin.set_duty(0);
                    ufmt::uwriteln!(&mut serial, "MOTOR stop\r").ok();
                }
            }
            None => {
                dir1.set_low();
                dir2.set_low();
                pwm_pin.set_duty(0);
                ufmt::uwriteln!(&mut serial, "DHT11 read error\r").ok();
            }
        }
    }
}