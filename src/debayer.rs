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

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use ser_io::{Bayer, Endianness, SerFile};

/// Trait for all debayering implementations
pub trait Debayer {
    fn debayer(&self, ser: &SerFile, frame_index: usize) -> Vec<u8>;
}

/// A very simple debayer that is easy to debug but inefficient and inaccurate
pub struct SimpleDebayer {}

impl Debayer for SimpleDebayer {
    fn debayer(&self, ser: &SerFile, frame_index: usize) -> Vec<u8> {
        let bytes = ser.read_frame(frame_index).unwrap();

        let width = ser.image_width;
        let height = ser.image_height;

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        let alpha = 255;

        let bytes_per_row = width * ser.bytes_per_pixel as u32;

        let base: i32 = 2;
        let max_value = base.pow(ser.pixel_depth_per_plane) as f32;

        let mut quad = [0_u16; 4];

        match ser.bayer {
            Bayer::RGGB => {
                let mut y = 0;
                while y < height {
                    let mut x = 0;

                    // each pixel on x axis has either 1 or 2 bytes
                    while x < width {
                        let y_offset = y * bytes_per_row;
                        let x_offset = x * ser.bytes_per_pixel as u32;
                        let offset = y_offset as usize + x_offset as usize;
                        let next_row_offset = (y_offset + bytes_per_row + x_offset) as usize;

                        if ser.bytes_per_pixel == 2 {
                            let mut pixel0 = &bytes[offset..offset + 2];
                            let mut pixel1 = &bytes[offset + 2..offset + 4];
                            let mut pixel2 = &bytes[next_row_offset..next_row_offset + 2];
                            let mut pixel3 = &bytes[next_row_offset + 2..next_row_offset + 4];

                            match ser.endianness {
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
        let debayer = SimpleDebayer {};
        let pixels = debayer.debayer(&ser, 0);
        assert_eq!(
            pixels.len() / 4,
            (ser.image_height as usize / 2) * (ser.image_width as usize / 2)
        );
    }
}
