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

use crate::video_format::Video;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use ser_io::{Bayer, Endianness};

/// Trait for all debayering implementations
pub trait ImageCodec {
    fn decode(&self, video: &dyn Video, frame_index: usize) -> (u32, u32, Vec<u8>);
}

pub struct RgbCodec {
    bayer: Bayer,
}

impl RgbCodec {
    pub fn new(bayer: Bayer) -> Self {
        Self { bayer }
    }
}

impl ImageCodec for RgbCodec {
    fn decode(&self, video: &dyn Video, frame_index: usize) -> (u32, u32, Vec<u8>) {
        let bytes = video.get_frame(frame_index).unwrap();
        let mut pixels =
            Vec::with_capacity((video.image_width() * video.image_height() * 4) as usize);
        let bytes_per_row = video.image_width() * 3;
        let alpha = 255;
        for y in 0..video.image_height() {
            for x in 0..video.image_width() {
                let y_offset = y * bytes_per_row;
                let x_offset = x * 3;
                let offset = y_offset as usize + x_offset as usize;

                let (r, g, b) = match self.bayer {
                    Bayer::BGR => {
                        let b = bytes[offset];
                        let g = bytes[offset + 1];
                        let r = bytes[offset + 2];
                        (r, g, b)
                    }
                    _ => todo!(),
                };

                // BGRa
                pixels.push(b);
                pixels.push(g);
                pixels.push(r);
                pixels.push(alpha);
            }
        }
        (video.image_width(), video.image_height(), pixels)
    }
}

/// A very simple debayer that is easy to debug but inefficient and inaccurate
pub struct DebayerCodec {}

impl ImageCodec for DebayerCodec {
    fn decode(&self, video: &dyn Video, frame_index: usize) -> (u32, u32, Vec<u8>) {
        let bytes = video.get_frame(frame_index).unwrap();

        let width = video.image_width();
        let height = video.image_height();

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        let alpha = 255;

        let base: i32 = 2;
        let max_value = base.pow(video.pixel_depth_bits()) as f32;

        let mut quad = [0_u16; 4];

        let bytes_per_row = width * video.bytes_per_pixel() as u32;
        let mut y = 0;
        while y < height {
            let mut x = 0;

            // each pixel on x axis has either 1 or 2 bytes
            while x < width {
                let y_offset = y * bytes_per_row;
                let x_offset = x * video.bytes_per_pixel() as u32;
                let offset = y_offset as usize + x_offset as usize;
                let next_row_offset = (y_offset + bytes_per_row + x_offset) as usize;

                if video.bytes_per_pixel() == 2 {
                    let mut pixel0 = &bytes[offset..offset + 2];
                    let mut pixel1 = &bytes[offset + 2..offset + 4];
                    let mut pixel2 = &bytes[next_row_offset..next_row_offset + 2];
                    let mut pixel3 = &bytes[next_row_offset + 2..next_row_offset + 4];

                    match video.endianness() {
                        Endianness::LittleEndian => {
                            quad[0] = pixel0.read_u16::<LittleEndian>().unwrap();
                            quad[1] = pixel1.read_u16::<LittleEndian>().unwrap();
                            quad[2] = pixel2.read_u16::<LittleEndian>().unwrap();
                            quad[3] = pixel3.read_u16::<LittleEndian>().unwrap();
                        }
                        Endianness::BigEndian => {
                            quad[0] = pixel0.read_u16::<BigEndian>().unwrap();
                            quad[1] = pixel1.read_u16::<BigEndian>().unwrap();
                            quad[2] = pixel2.read_u16::<BigEndian>().unwrap();
                            quad[3] = pixel3.read_u16::<BigEndian>().unwrap();
                        }
                    }
                } else {
                    let mut pixel0 = &bytes[offset..offset + 1];
                    let mut pixel1 = &bytes[offset + 1..offset + 2];
                    let mut pixel2 = &bytes[next_row_offset..next_row_offset + 1];
                    let mut pixel3 = &bytes[next_row_offset + 1..next_row_offset + 2];

                    quad[0] = pixel0.read_u8().unwrap() as u16;
                    quad[1] = pixel1.read_u8().unwrap() as u16;
                    quad[2] = pixel2.read_u8().unwrap() as u16;
                    quad[3] = pixel3.read_u8().unwrap() as u16;
                }

                // this is not real debayering, just using raw values without interpolation
                let r = quad[0];
                let g = quad[1];
                let b = quad[3];

                // BGRA
                pixels.push(((b as f32 / max_value) * 255.0) as u8);
                pixels.push(((g as f32 / max_value) * 255.0) as u8);
                pixels.push(((r as f32 / max_value) * 255.0) as u8);
                pixels.push(alpha);

                x += 2;
            }
            y += 2;
        }
        (width / 2, height / 2, pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avi::AviFile;
    use crate::video_format::{AviVideo, SerVideo};
    use ser_io::SerFile;

    #[test]
    fn test_decode_avi() {
        // AVI from ZWO ASI 224 MC
        let video: Box<dyn Video> = Box::new(AviVideo {
            avi: AviFile::open("/home/andy/Documents/2021-09-05-0312_7-CapObj.AVI").unwrap(),
        });
        assert_eq!(1304, video.image_width());
        assert_eq!(976, video.image_height());
        assert_eq!(44, video.frame_count());
        assert_eq!(1, video.bytes_per_pixel());
        assert_eq!(8, video.pixel_depth_bits());
        let frame0 = video.get_frame(0).unwrap();
        assert_eq!(1304 * 976 * 3, frame0.len());

        /*
        AviMainHeader {
            micro_sec_per_frame: 333333,
            max_bytes_per_sec: 0,
            padding_granularity: 512,
            flags: 2064,
            total_frames: 44,
            initial_frames: 0, streams: 1,
            suggested_buffer_size: 3819000,
            width: 1304, height: 976, reserved: [0, 0, 0, 0] }
        AviStreamHeader {
            fcc_type: [118, 105, 100, 115],
            fcc_handler: [68, 73, 66, 32], // "hsf2"
            flags: 0,
            priority: 0,
            language: 0, initial_frames: 0, scale: 3333333, rate: 10000000, start: 0, length: 44,
            suggested_buffer_size: 3819000, quality: 0,
            sample_size: 0, left: 0, top: 0, right: 0, bottom: 0 }
        BitMapInfo {
            header: BitMapInfoHeader {
                size: 40,
                width: 1304,
                height: -976,
                planes: 1,
                bit_count: 24,
                compression: 0,
                size_image: 3818112,
                x_pels_per_meter: 4294967297,
                y_pels_per_meter: 0,
                clr_used: 0,
                clr_important: 65793 },
            rgb: RgbQuad { blue: 2, green: 2, red: 2, reserved: 0 }
        }

         */

        let codec = RgbCodec::new(Bayer::BGR);
        let (w, h, pixels) = codec.decode(video.as_ref(), 0);
        assert_eq!(1304, w);
        assert_eq!(976, h);
        assert_eq!(
            pixels.len(),
            video.image_height() as usize * video.image_width() as usize * 4
        );
    }

    #[test]
    fn test_decode_ser() {
        // RAW16 SER from ZWO ASI 294 MC
        let ser = SerFile::open("/home/andy/Documents/2021-09-20-0323_1-CapObj.SER").unwrap();
        assert_eq!(4144, ser.image_width);
        assert_eq!(2822, ser.image_height);
        assert_eq!(4144 * 2822 * 2, ser.image_frame_size);
        assert_eq!(2, ser.bytes_per_pixel);

        let video: Box<dyn Video> = Box::new(SerVideo { ser });
        assert_eq!(4144, video.image_width());
        assert_eq!(2822, video.image_height());
        assert_eq!(100, video.frame_count());
        assert_eq!(2, video.bytes_per_pixel());
        assert_eq!(16, video.pixel_depth_bits());

        let codec = DebayerCodec {};
        let (w, h, pixels) = codec.decode(video.as_ref(), 0);
        assert_eq!(4144 / 2, w);
        assert_eq!(2822 / 2, h);
        assert_eq!(
            pixels.len() / 4,
            (video.image_height() as usize / 2) * (video.image_width() as usize / 2)
        );
    }
}
