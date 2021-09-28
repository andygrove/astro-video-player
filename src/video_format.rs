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

use std::io::Result;

use crate::avi::AviFile;
use ser_io::{Bayer, Endianness, SerFile};

pub trait Video {
    fn image_width(&self) -> u32;
    fn image_height(&self) -> u32;
    fn frame_count(&self) -> usize;
    fn bytes_per_pixel(&self) -> u8;
    fn pixel_depth_bits(&self) -> u32;
    fn bayer(&self) -> &Bayer;
    fn endianness(&self) -> &Endianness;
    fn get_frame(&self, index: usize) -> Result<&[u8]>;
}

pub struct SerVideo {
    pub ser: SerFile,
}

impl Video for SerVideo {
    fn image_width(&self) -> u32 {
        self.ser.image_width
    }

    fn image_height(&self) -> u32 {
        self.ser.image_height
    }

    fn frame_count(&self) -> usize {
        self.ser.frame_count
    }

    fn get_frame(&self, index: usize) -> Result<&[u8]> {
        self.ser.read_frame(index)
    }

    fn bytes_per_pixel(&self) -> u8 {
        self.ser.bytes_per_pixel
    }

    fn pixel_depth_bits(&self) -> u32 {
        self.ser.pixel_depth_per_plane
    }
    fn bayer(&self) -> &Bayer {
        &self.ser.bayer
    }

    fn endianness(&self) -> &Endianness {
        &self.ser.endianness
    }
}

pub struct AviVideo {
    pub avi: AviFile,
}

impl Video for AviVideo {
    fn image_width(&self) -> u32 {
        self.avi.main_header().width
    }

    fn image_height(&self) -> u32 {
        self.avi.main_header().height
    }

    fn frame_count(&self) -> usize {
        self.avi.main_header().total_frames as usize
    }

    fn bytes_per_pixel(&self) -> u8 {
        //TODO
        1
    }

    fn pixel_depth_bits(&self) -> u32 {
        //TODO
        8
        //self.avi.stream_format().header.bit_count as u32
    }

    fn bayer(&self) -> &Bayer {
        //TODO
        &Bayer::BGR
    }

    fn endianness(&self) -> &Endianness {
        //TODO
        &Endianness::LittleEndian
    }

    fn get_frame(&self, index: usize) -> Result<&[u8]> {
        let frame_meta = &self.avi.frames()[index];
        Ok(self.avi.read_bytes(frame_meta))
    }
}
