//! viewer for wio_umwelt_monitor

use embedded_graphics as eg;
use panic_halt as _;
use wio_terminal as wio;

use eg::{fonts::*, pixelcolor::*, prelude::*, primitives::*, style::*};
use core::fmt::Write;
use heapless::consts::*;
use heapless::String;


// Defined constant values
pub const INVALID_DAT_NUM: f32 = 999.9;
pub const WINDOW_WIDTH: usize = 320;
//pub const WINDOW_HEIGHT: usize = 240; // unused variable

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SensorType {
    Temperature,
    Humidity,
    Co2Concentration,
    AtmPressure
}

pub struct DataHistory {
    dat: [f32; WINDOW_WIDTH +1],
    max: f32,
    min: f32
}

pub struct DataSet {
    tmp: DataHistory,
    hum: DataHistory,
    atm: DataHistory,
    co2: DataHistory
}

pub struct NumberPrintElement {
    var: f32,
    x_r: i32,
    y: i32,
    recent: i32,
    last: i32
}

#[derive(Debug, Copy, Clone)]
pub struct Coordinates {
    title_x: i32,
    unit_x: i32,
    num_x_r: i32,
    tmp_y: i32,
    hum_y: i32,
    co2_y: i32,
    atm_y: i32,
    graph_y: i32,
    graph_height: i32
}

pub struct Viewer {
    pos: Coordinates,
    mode: SensorType,
    num_tmp: NumberPrintElement,
    num_hum: NumberPrintElement,
    num_co2: NumberPrintElement,
    num_atm: NumberPrintElement,
    history: DataSet
}

impl DataHistory {
    pub fn new() -> DataHistory {
        DataHistory {
            dat : [INVALID_DAT_NUM; WINDOW_WIDTH +1],
            max : 0.0,
            min : 0.0
        }
    }

    pub fn set_new_data(&mut self, new_data: f32) {

        self.max = new_data;
        self.min = new_data;
        self.dat[WINDOW_WIDTH] = new_data;

        for i in 0..WINDOW_WIDTH {
            self.dat[i] = self.dat[i+1];
            if INVALID_DAT_NUM != self.dat[i] {
                if self.max < self.dat[i] {
                    self.max = self.dat[i];
                }
                if self.dat[i] < self.min {
                    self.min = self.dat[i];
                }
            }
        }
    }

    pub fn get_rate(&self, itr: usize) -> f32 {

        if (self.dat[itr] != INVALID_DAT_NUM) && (0.0 < self.max - self.min) {
            (self.dat[itr] - self.min) / (self.max - self.min)
        }
        else {
            0.0
        }
    }
}

impl DataSet {
    pub fn new() ->DataSet {
        DataSet {
            tmp: DataHistory::new(),
            hum: DataHistory::new(),
            atm: DataHistory::new(),
            co2: DataHistory::new()
        }
    }

    pub fn set_new_data(&mut self, sensor: SensorType, value: f32) {
        match sensor {
            SensorType::Temperature => self.tmp.set_new_data(value),
            SensorType::Humidity => self.hum.set_new_data(value),
            SensorType::Co2Concentration => self.co2.set_new_data(value),
            SensorType::AtmPressure => self.atm.set_new_data(value)
        }
    }

    pub fn get_rate(&self, sensor: SensorType, itr: usize) -> f32 {
        match sensor {
            SensorType::Temperature => self.tmp.get_rate(itr),
            SensorType::Humidity => self.hum.get_rate(itr),
            SensorType::Co2Concentration => self.co2.get_rate(itr),
            SensorType::AtmPressure => self.atm.get_rate(itr)
        }
    }
}

impl NumberPrintElement {
    pub fn new(x_right: i32, y: i32) -> NumberPrintElement {
        NumberPrintElement {
            var: INVALID_DAT_NUM,
            x_r: x_right,
            y: y,
            recent: 0,
            last: 0
        }
    }

    pub fn print(&mut self, display: &mut wio::LCD, value: f32, color: Rgb565) {

        // 表示範囲を前回と比較するための小数点以下第2位を四捨五入して10倍した値
        self.recent = (10.0 * value + 0.5) as i32;

        // 値に変化があったときだけ表示を更新する
        if self.recent != self.last {

            self.print_sub(display, Rgb565::BLACK);
            self.var = value;
            self.print_sub(display, color);

            self.last = self.recent;
        }
    }

    //  右詰め小数点以下1桁で数値を表示
    fn print_sub(&mut self, display: &mut wio::LCD, color: Rgb565) {

        if self.var != INVALID_DAT_NUM {
            let mut textbuf = String::<U32>::new();
            write!(&mut textbuf, "{:.1}", self.var).unwrap();

            let x_l = self.x_r - (textbuf.len() as i32) * 25;

            Text::new(textbuf.as_str(), Point::new(x_l, self.y))
                .into_styled(TextStyle::new(Font24x32, color))
                .draw(display)
                .unwrap();
        }
    }
}

impl Coordinates {
    pub fn new(x_title: i32, x_unit: i32, x_num_r: i32, y_tmp: i32, y_hum: i32, y_co2: i32, y_atm: i32, y_graph: i32, height_graph: i32)-> Coordinates {
        Coordinates {
            title_x: x_title,
            unit_x: x_unit,
            num_x_r: x_num_r,
            tmp_y: y_tmp,
            hum_y: y_hum,
            co2_y: y_co2,
            atm_y: y_atm,
            graph_y: y_graph,
            graph_height: height_graph,
        }
    }
}

impl Viewer {
    pub fn new(cordinates: Coordinates)-> Viewer {
        Viewer {
            pos: cordinates,
            mode: SensorType::Co2Concentration,
            num_tmp: NumberPrintElement::new(cordinates.num_x_r, cordinates.tmp_y),
            num_hum: NumberPrintElement::new(cordinates.num_x_r, cordinates.hum_y),
            num_co2: NumberPrintElement::new(cordinates.num_x_r, cordinates.co2_y),
            num_atm: NumberPrintElement::new(cordinates.num_x_r, cordinates.atm_y),
            history: DataSet::new()
        }
    }

    pub fn update(&mut self, display: &mut wio::LCD, tmp: f32, hum: f32, co2: f32, atm: f32) {
        self.history.set_new_data(SensorType::Temperature, tmp);
        self.history.set_new_data(SensorType::Humidity, hum);
        self.history.set_new_data(SensorType::Co2Concentration, co2);
        self.history.set_new_data(SensorType::AtmPressure, atm);

        let color_co2 = if 1000.0 <= co2 {Rgb565::RED} else {get_color(SensorType::Co2Concentration)};
        self.num_tmp.print(display, tmp, get_color(SensorType::Temperature));
        self.num_hum.print(display, hum, get_color(SensorType::Humidity));
        self.num_co2.print(display, co2, color_co2);
        self.num_atm.print(display, atm, get_color(SensorType::AtmPressure));
        self.write_graph(display);
    }

    pub fn next_mode (&mut self, display: &mut wio::LCD) {
        self.mode = match self.mode {
            SensorType::Temperature => SensorType::Humidity,
            SensorType::Humidity => SensorType::Co2Concentration,
            SensorType::Co2Concentration => SensorType::AtmPressure,
            SensorType::AtmPressure => SensorType::Temperature
        };

        self.write_graph(display);
    }

    // 数値以外の変動しない表示を描画
    pub fn print_labels(&mut self, display: &mut wio::LCD) {

        Text::new("Temp.", Point::new(self.pos.title_x, self.pos.tmp_y))
            .into_styled(TextStyle::new(Font12x16, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new(".", Point::new(self.pos.unit_x-5, self.pos.tmp_y-25))
            .into_styled(TextStyle::new(Font24x32, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("C", Point::new(self.pos.unit_x+8, self.pos.tmp_y))
            .into_styled(TextStyle::new(Font24x32, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("Humid.", Point::new(self.pos.title_x, self.pos.hum_y))
            .into_styled(TextStyle::new(Font12x16, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("%", Point::new(self.pos.unit_x, self.pos.hum_y))
            .into_styled(TextStyle::new(Font24x32, Rgb565::WHITE))
            .draw(display)
            .unwrap();
        Text::new("CO2", Point::new(self.pos.title_x, self.pos.co2_y))
            .into_styled(TextStyle::new(Font12x16, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("ppm", Point::new(self.pos.unit_x, self.pos.co2_y))
            .into_styled(TextStyle::new(Font24x32, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("Atm.", Point::new(self.pos.title_x, self.pos.atm_y))
            .into_styled(TextStyle::new(Font12x16, Rgb565::WHITE))
            .draw(display)
            .unwrap();

        Text::new("hPa", Point::new(self.pos.unit_x, self.pos.atm_y))
            .into_styled(TextStyle::new(Font24x32, Rgb565::WHITE))
            .draw(display)
            .unwrap();
    }

    // グラフエリアの描画
    fn write_graph(&mut self, display: &mut wio::LCD) {

        let y_bottom: i32 = self.pos.graph_y + self.pos.graph_height;
        let color = get_color( self.mode);

        let style =  PrimitiveStyleBuilder::new()
            .fill_color(color)
            .build();

        let style_black = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::BLACK)
            .build();

        let style_red = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::RED)
            .build();

        for i in 0..320 {
            let mut value = (self.pos.graph_height as f32 * self.history.get_rate(self.mode, i as usize)) as i32;

            if value < 0 {
                value = 0;
            }

            let bar_reset =
                Rectangle::new(Point::new(i, self.pos.graph_y), Point::new(i, y_bottom-value))
                    .into_styled(style_black);
            bar_reset.draw(display).unwrap();

            if 0 < value {
                let bar = if (self.mode == SensorType::Co2Concentration) && (1000.0 <= self.history.co2.dat[i as usize]) {
                    Rectangle::new(Point::new(i, y_bottom-value), Point::new(i, y_bottom))
                        .into_styled(style_red)
                }
                else {
                    Rectangle::new(Point::new(i, y_bottom-value), Point::new(i, y_bottom))
                        .into_styled(style)
                };

                bar.draw(display).unwrap();
            }
        }
    }
}

// 画面の初期化
pub fn print_initializing(display: &mut wio::LCD, is_initialized: bool) {

    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();

    // LCDを黒色で塗りつぶす
    let background =
        Rectangle::new(Point::new(0, 0), Point::new(319, 239))
            .into_styled(style);
    background.draw(display).unwrap();

    if is_initialized {
        // 初期化中表示
        Text::new("Initializing...", Point::new(70, 210))
            .into_styled(TextStyle::new(Font12x16, Rgb565::GREEN))
            .draw(display)
            .unwrap();
    }
    else {
        Text::new("initialization failure", Point::new(30, 210))
            .into_styled(TextStyle::new(Font12x16, Rgb565::RED))
            .draw(display)
            .unwrap();
    }
}

// 各センサに対応した色
pub fn get_color(sensor: SensorType) -> Rgb565 {
    match sensor {
        SensorType::Temperature => {Rgb565::MAGENTA},
        SensorType::Humidity => {Rgb565::CYAN},
        SensorType::Co2Concentration => {Rgb565::GREEN},
        SensorType::AtmPressure => {Rgb565::new(0x1c, 0x28, 0x1f)}
    }
}
