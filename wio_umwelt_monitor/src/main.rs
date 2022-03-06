//! main for wio_umwelt_monitor

#![no_std]
#![no_main]

use panic_halt as _;
use wio_terminal as wio;

use cortex_m::peripheral::NVIC;
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::{entry, Pins};
use wio::hal::gpio::*;
use wio::hal::sercom::*;
use wio::hal::timer::TimerCounter;
use wio::pac::{CorePeripherals, Peripherals, interrupt, TC3};
use wio::prelude::*;


use scd30::*;
use bm1383aglv::*;

mod viewer;
use viewer::*;

// defined constant value
const SENSING_INTERVAL: u16 = 12;


// main()関数と割り込みハンドラとで共有するリソース
struct Ctx {
    tc3: TimerCounter<TC3>
}
static mut CTX: Option<Ctx> = None;
static mut SECOND: u16 = 0;


#[entry]
fn main() -> ! {
    const X_TITLE: i32 = 10;
    const X_UNIT: i32 = 240;
    const X_NUM_R: i32 = 230;
    const Y_TMP: i32 = 5;
    const Y_HUM: i32 = 55;
    const Y_CO2: i32 = 105;
    const Y_ATM: i32 = 155;
    const Y_GRAPH: i32 = 199;
    const HEIGHT_GRAPH: i32 = 40;

    let coordinates = Coordinates::new(X_TITLE, X_UNIT, X_NUM_R, Y_TMP, Y_HUM, Y_CO2, Y_ATM, Y_GRAPH, HEIGHT_GRAPH);

    let mut view: Viewer = Viewer::new(coordinates);

    loop {
        umwelt_monitor(&mut view);
    }
}

fn umwelt_monitor(view: &mut Viewer) {

    let mut peripherals = Peripherals::take().unwrap();
    let mut pins = Pins::new(peripherals.PORT);

    // 3.3VをPinから出力するように設定
    let mut output_ctr = pins.output_ctr_3v3.into_push_pull_output(&mut pins.port);
    output_ctr.set_low().unwrap();

    // LEDを出力に設定
    let mut led = pins.user_led.into_push_pull_output(&mut pins.port);

    // ボタン
    let button = pins.switch_z.into_floating_input(&mut pins.port);
    let button_right = pins.button1.into_floating_input(&mut pins.port);
    let button_center = pins.button2.into_floating_input(&mut pins.port);
    let button_left = pins.button3.into_floating_input(&mut pins.port);

    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);

    // ディスプレイドライバを初期化する
    let display = wio::Display {
        miso: pins.lcd_miso,
        mosi: pins.lcd_mosi,
        sck: pins.lcd_sck,
        cs: pins.lcd_cs,
        dc: pins.lcd_dc,
        reset: pins.lcd_reset,
        backlight: pins.lcd_backlight
    };

    let (mut display, mut backlight) = display.init(
        &mut clocks,
        peripherals.SERCOM7,
        &mut peripherals.MCLK,
        &mut pins.port,
        58.mhz(),
        &mut delay
    ).unwrap();

    // I2Cドライバオブジェクトを初期化する
    let gclk0 = &clocks.gclk0();
    let mut i2c: I2CMaster3<Sercom3Pad0<Pa17<PfD>>, Sercom3Pad1<Pa16<PfD>>> = I2CMaster3::new(
        &clocks.sercom3_core(&gclk0).unwrap(),
        400.khz(),
        peripherals.SERCOM3,
        &mut peripherals.MCLK,
        pins.i2c1_sda.into_pad(&mut pins.port),
        pins.i2c1_scl.into_pad(&mut pins.port)
    );

    // CO2センサを初期化する
    let mut sensor = SCD30::new();
    let mut is_sensor_initialized = true;
    if let Err(_) = sensor.init(&mut i2c, SENSING_INTERVAL){
        is_sensor_initialized = false;
    }

    if is_sensor_initialized {
        if let Err(_) = sensor.set_auto_calibration(&mut i2c, true){
            is_sensor_initialized = false;
        }
    }

    // 気圧センサを初期化する
    let mut barometer = BM1383AGLV::new();
    let mut is_barometer_enabled = true;
    if let Err(_code) = barometer.init(&mut i2c, &mut delay){
        is_barometer_enabled = false;
    }

    print_initializing(&mut display, is_sensor_initialized);

    if !is_sensor_initialized {
        // 初期化失敗時はここで止めてしまう
        loop {}
    }

    // 数値以外の変動しない表示を描画
    view.print_labels(&mut display);

    // 2MHzのクロックを取得する
    let gclk5 = clocks
        .get_gclk(wio::pac::gclk::pchctrl::GEN_A::GCLK5)
        .unwrap();
    // TC3へのクロックを2MHzにする
    let timer_clock = clocks.tc2_tc3(&gclk5).unwrap();
    // TC3ドライバオブジェクトを初期化する
    let mut tc3 = TimerCounter::tc3_(
        &timer_clock,
        peripherals.TC3,
        &mut peripherals.MCLK,
    );

    // 割り込みコントローラで、TC3の割り込み通知を有効化する
    unsafe {
        NVIC::unmask(interrupt::TC3);
    }

    // 1秒のカウントを開始して、TC3が割り込みが発生するようにする
    tc3.start(1.s());
    tc3.enable_interrupt();

    // 割り込みハンドラと共有するリソースを格納する
    unsafe {
        CTX = Some(Ctx {
            tc3
        });
    }

    let mut is_lcd_on = true;
    let mut updated_second:u16 = 0;

    loop {
        led.set_high().unwrap();

        let (is_available, tmp, hum, co2, atm) = get_sensor_value(&mut i2c, &mut sensor, &mut barometer, is_barometer_enabled);
        if is_available {
            view.update(&mut display, tmp, hum, co2, atm);
        }

        led.set_low().unwrap();

        loop {
            if is_lcd_on {
                if button_right.is_low().unwrap() {
                    backlight.set_low().unwrap();
                    is_lcd_on = false;
                }
                if button.is_low().unwrap() {
                    view.next_mode(&mut display);
                }
            }
            else {
                if button_right.is_low().unwrap() || button_center.is_low().unwrap() || button_left.is_low().unwrap() || button.is_low().unwrap() {
                    backlight.set_high().unwrap();
                    is_lcd_on = true;
                }
            }

            unsafe {
                if (SECOND % SENSING_INTERVAL == 0) && (updated_second != SECOND) {
                    updated_second = SECOND;
                    break;
                }
            }

            delay.delay_ms(250u16);
        }
    }
}

// センサデータの取得
pub fn get_sensor_value( i2c: &mut I2CMaster3<Sercom3Pad0<Pa17<PfD>>,
                         Sercom3Pad1<Pa16<PfD>>>,
                         sensor: &mut SCD30,
                         barometer: &mut BM1383AGLV,
                         is_barometer_available: bool
                        )
    -> (bool, f32, f32, f32, f32)
{
    let mut tmp: f32 = 0.0;
    let mut hum: f32 = 0.0;
    let mut co2: f32 = 0.0;
    let mut atm: f32 = 0.0;
    let mut valid = false;

    if let Ok(is_available) = sensor.is_available(i2c) {
        if is_available {
            if let Ok((get_co2, get_tmp, get_hum)) = sensor.get_value(i2c) {
                if get_co2 < 100.0 {
                    // なぜかまともなデータが取れないときは無視
                }
                else {
                    valid = true;
                    tmp = get_tmp;
                    hum = get_hum;
                    co2 = get_co2;

                    if is_barometer_available {
                        if let Ok((_tmp, get_atm)) = barometer.get_value(i2c) {

                            atm = get_atm;
                        }
                        else {
                            valid = false;
                        }
                    }
                }
            }
        }
    }

    (valid, tmp, hum, co2, atm)
}

// TC3の割り込みハンドラ（1秒ごとに呼ばれる）
#[interrupt]
fn TC3() {
    unsafe {
        let ctx = CTX.as_mut().unwrap();

        SECOND = (SECOND + 1) % (SENSING_INTERVAL * 5);

        // 次のカウントを開始する
        ctx.tc3.wait().unwrap();
    }
}
