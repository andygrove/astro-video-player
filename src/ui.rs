// MIT License
//
// Copyright (c) 2021 Andy Grove
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use iced::image::Handle;
use iced::{
    button, Align, Application, Button, Clipboard, Column, Container, Element, Image, Length, Row,
    Text,
};
use iced::{executor, Command};

use crate::codec::ImageCodec;
use crate::video_format::Video;

pub struct VideoPlayerArgs {
    pub video: Option<Box<dyn Video>>,
    pub codec: Option<Box<dyn ImageCodec>>,
}

impl Default for VideoPlayerArgs {
    fn default() -> Self {
        Self {
            video: None,
            codec: None,
        }
    }
}

pub struct VideoPlayer {
    video: Box<dyn Video>,
    codec: Box<dyn ImageCodec>,
    value: u32,
    increment_button: button::State,
    decrement_button: button::State,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    NextFrame,
    PrevFrame,
}

impl Application for VideoPlayer {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = VideoPlayerArgs;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let app = Self {
            video: flags.video.unwrap(),
            codec: flags.codec.unwrap(),
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
                if self.value + 1 < self.video.frame_count() as u32 {
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
        let index = if (self.value as usize) < self.video.frame_count() {
            self.value as usize
        } else {
            self.video.frame_count() - 1
        };

        let (w, h, pixels) = self.codec.decode(self.video.as_ref(), index);

        let handle = Handle::from_pixels(w, h, pixels);

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
                    self.video.frame_count()
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
