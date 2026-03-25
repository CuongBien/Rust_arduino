#![no_std]
#![no_main]

use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Khởi tạo Serial với tốc độ 57600 baud
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    ufmt::uwriteln!(&mut serial, "Chào bạn! Arduino chạy bằng Rust đây.\r")
        .unwrap_or_else(|_err| panic!());

    loop {
        ufmt::uwriteln!(&mut serial, "Đang chạy vòng lặp...\r")
            .unwrap_or_else(|_err| panic!());
        arduino_hal::delay_ms(1000);
    }
}