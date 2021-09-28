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

use iced::{Application, Settings};
use structopt::StructOpt;

use astro_video_player::avi::AviFile;
use astro_video_player::ui::VideoPlayer;
use astro_video_player::ui::VideoPlayerArgs;
use astro_video_player::video_format::{AviVideo, SerVideo};
use ser_io::{Bayer, SerFile};

#[derive(StructOpt, Debug)]
struct Opt {
    filename: String,
}

pub fn main() -> iced::Result {
    let opt = Opt::from_args();

    if opt.filename.ends_with(".AVI") {
        let avi = AviFile::open(&opt.filename).unwrap();
        println!("{:?}", avi.main_header());
        println!("{:?}", avi.stream_header());
        println!("{:?}", avi.stream_format());
        println!("avi has {} frames", avi.frames().len());

        let mut settings: Settings<VideoPlayerArgs> = Settings::default();
        settings.flags.ser = Some(Box::new(AviVideo { avi }));
        VideoPlayer::run(settings)
    } else if opt.filename.ends_with(".SER") {
        match SerFile::open(&opt.filename) {
            Ok(ser) => match ser.bayer {
                Bayer::RGGB => {
                    let mut settings: Settings<VideoPlayerArgs> = Settings::default();
                    settings.flags.ser = Some(Box::new(SerVideo { ser }));
                    VideoPlayer::run(settings)
                }
                other => {
                    println!("Unsupported bayer {:?}", other);
                    Ok(())
                }
            },
            Err(e) => {
                println!("Could not open SER file: {:?}", e);
                Ok(())
            }
        }
    } else {
        println!("Can only read AVI and SER");
        Ok(())
    }
}
