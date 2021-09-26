use byteorder::{LittleEndian, ReadBytesExt};
use iced::image::Handle;
use iced::{
    button, Align, Application, Button, Clipboard, Column, Container, Element, Image, Length, Row,
    Settings, Text,
};
use iced::{executor, Command};
use ser_io::{Bayer, SerFile};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    filename: String,
}

pub fn main() -> iced::Result {
    let opt = Opt::from_args();
    let mut settings: Settings<MyFlags> = Settings::default();
    settings.flags.filename = opt.filename;
    VideoPlayer::run(settings)
}

struct MyFlags {
    filename: String,
}

impl Default for MyFlags {
    fn default() -> Self {
        Self {
            filename: "".to_string(),
        }
    }
}

struct VideoPlayer {
    ser: SerFile,
    value: u32,
    increment_button: button::State,
    decrement_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    NextFrame,
    PrevFrame,
}

impl Application for VideoPlayer {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = MyFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let ser = SerFile::open(&flags.filename).unwrap();

        let app = Self {
            ser,
            value: 0,
            increment_button: button::State::default(),
            decrement_button: button::State::default(),
        };

        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("Astro Video Player")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match message {
            Message::NextFrame => {
                if self.value + 1 < self.ser.frame_count as u32 {
                    self.value += 1;
                }
            }
            Message::PrevFrame => {
                if self.value > 0 {
                    self.value -= 1;
                }
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        let debayer = DumbDebayer {};

        let index = if (self.value as usize) < self.ser.frame_count {
            self.value as usize
        } else {
            self.ser.frame_count - 1
        };

        let pixels = debayer.debayer(&self.ser, index);

        let handle =
            Handle::from_pixels(self.ser.image_width / 2, self.ser.image_height / 2, pixels);

        let image = Image::new(handle).width(Length::Fill).height(Length::Fill);

        let controls = Row::new()
            .padding(20)
            .align_items(Align::Center)
            .push(
                Button::new(&mut self.decrement_button, Text::new("<<"))
                    .on_press(Message::PrevFrame),
            )
            .push(
                Text::new(format!(
                    "Frame {} of {}",
                    self.value + 1,
                    self.ser.frame_count
                ))
                .size(22),
            )
            .push(
                Button::new(&mut self.increment_button, Text::new(">>"))
                    .on_press(Message::NextFrame),
            );

        Column::new()
            .padding(20)
            .align_items(Align::Center)
            .push(
                Container::new(image)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .push(controls)
            .into()
    }
}

trait Debayer {
    fn debayer(&self, ser: &SerFile, frame_index: usize) -> Vec<u8>;
}

struct DumbDebayer {}

impl Debayer for DumbDebayer {
    fn debayer(&self, ser: &SerFile, frame_index: usize) -> Vec<u8> {
        let bytes = ser.read_frame(frame_index).unwrap();

        //TODO assumes RAW16 !
        //TODO assumes little-endian !

        let w = ser.image_width;
        let h = ser.image_height;

        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        let alpha = 255;

        let bytes_per_row = w * ser.bytes_per_pixel as u32;

        let base: i32 = 2;
        let max_value = base.pow(ser.pixel_depth_per_plane) as f32;

        match ser.bayer {
            Bayer::RGGB => {
                let mut y = 0;
                while y < h {
                    let mut x = 0;

                    // each pixel on x axis has either 1 or 2 bytes
                    while x < w {
                        let y_offset = y * bytes_per_row;
                        let x_offset = x * ser.bytes_per_pixel as u32;
                        let offset = y_offset as usize + x_offset as usize;
                        let next_row_offset = (y_offset + bytes_per_row + x_offset) as usize;

                        // this is not real debayering, just using raw values without interpolation
                        let mut r = &bytes[offset..offset + 2];
                        let mut g = &bytes[offset + 2..offset + 4];
                        let mut b = &bytes[next_row_offset + 2..next_row_offset + 4];

                        let r = r.read_u16::<LittleEndian>().unwrap();
                        let g = g.read_u16::<LittleEndian>().unwrap();
                        let b = b.read_u16::<LittleEndian>().unwrap();

                        // BGRA
                        pixels.push(((b as f32 / max_value) * 255.0) as u8);
                        pixels.push(((g as f32 / max_value) * 255.0) as u8);
                        pixels.push(((r as f32 / max_value) * 255.0) as u8);
                        pixels.push(alpha);

                        x += ser.bytes_per_pixel as u32;
                    }
                    y += 2;
                }
            }
            _ => todo!("bayer not supported yet"),
        }

        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_image() {
        let ser = SerFile::open("/home/andy/Documents/2021-09-20-0323_1-CapObj.SER").unwrap();
        let debayer = DumbDebayer {};
        let pixels = debayer.debayer(&ser, 0);
        assert_eq!(
            pixels.len() / 4,
            (ser.image_height as usize / 2) * (ser.image_width as usize / 2)
        );
    }
}