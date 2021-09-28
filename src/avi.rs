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

/*

main_header: AviMainHeader { micro_sec_per_frame: 333333, max_bytes_per_sec: 0, padding_granularity: 512, flags: 2064, total_frames: 44, initial_frames: 0, streams: 1, suggested_buffer_size: 3819000, width: 1304, height: 976, reserved: [0, 0, 0, 0] }
stream_header: AviStreamHeader { fcc_type: [118, 105, 100, 115], fcc_handler: [68, 73, 66, 32], flags: 0, priority: 0, language: 0, initial_frames: 0, scale: 3333333, rate: 10000000, start: 0, length: 44, suggested_buffer_size: 3819000, quality: 0, sample_size: 0, left: 0, top: 0, right: 0, bottom: 0 }
stream_format: BitMapInfo { header: BitMapInfoHeader { size: 40, width: 1304, height: -976, planes: 1, bit_count: 24, compression: 0, size_image: 3818112, x_pels_per_meter: 4294967297, y_pels_per_meter: 0, clr_used: 0, clr_important: 65793 }, rgb: RgbQuad { blue: 2, green: 2, red: 2, reserved: 0 } }


 */

use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};
use std::str;
use std::{error, fmt};

use riff_io::{ChunkMeta, Entry, FourCC, ListMeta, RiffFile};

// use https://www.rapidtables.com/convert/number/ascii-to-hex.html

const FOURCC_AVIH: FourCC = [0x61, 0x76, 0x69, 0x68];
const FOURCC_JUNK: FourCC = [0x4a, 0x55, 0x4e, 0x4b];
const FOURCC_HDRL: FourCC = [0x68, 0x64, 0x72, 0x6c];
const FOURCC_STRH: FourCC = [0x73, 0x74, 0x72, 0x68];
const FOURCC_STRF: FourCC = [0x73, 0x74, 0x72, 0x66];
const FOURCC_STRL: FourCC = [0x73, 0x74, 0x72, 0x6c];
const FOURCC_MOVI: FourCC = [0x6d, 0x6f, 0x76, 0x69];

pub struct AviFile {
    riff: RiffFile,
    main_header: AviMainHeader,
    stream_header: AviStreamHeader,
    stream_format: BitMapInfo,
    /// chunk meta for the image frames
    frames: Vec<ChunkMeta>,
}

impl AviFile {
    pub fn open(filename: &str) -> Result<Self> {
        let riff = RiffFile::open(filename)?;
        let entries = riff.read_entries()?;

        // main header
        let hdrl = find_mandatory_list(&entries, FOURCC_HDRL)?;
        let chunk = find_mandatory_chunk(hdrl, FOURCC_AVIH)?;
        let main_header = parse_main_header(&riff, chunk)?;

        // get first stream header and format (only one stream is currently supported)
        let strl = find_mandatory_list_in_list(hdrl, FOURCC_STRL)?;
        let strh = find_mandatory_chunk(strl, FOURCC_STRH)?;
        let strf = find_mandatory_chunk(strl, FOURCC_STRF)?;
        let stream_header = parse_stream_header(&riff, strh)?;
        let stream_format = parse_stream_format(&riff, strf)?;

        // video frames
        let movi = find_mandatory_list(&entries, FOURCC_MOVI)?;

        let frames = movi
            .children
            .iter()
            .filter_map(|e| match e {
                Entry::Chunk(chunk) if chunk.chunk_id == [0x30, 0x30, 0x64, 0x62] => {
                    Some(chunk.clone())
                }
                _ => None,
            })
            .collect();

        Ok(Self {
            riff,
            main_header,
            stream_header,
            stream_format,
            frames,
        })
    }

    pub fn main_header(&self) -> &AviMainHeader {
        &self.main_header
    }

    pub fn stream_header(&self) -> &AviStreamHeader {
        &self.stream_header
    }

    pub fn stream_format(&self) -> &BitMapInfo {
        &self.stream_format
    }

    pub fn frames(&self) -> &[ChunkMeta] {
        &self.frames
    }

    pub fn read_bytes(&self, chunk_meta: &ChunkMeta) -> &[u8] {
        self.riff
            .read_bytes(chunk_meta.data_offset..chunk_meta.data_offset + chunk_meta.data_size)
    }
}

fn parse_main_header(riff: &RiffFile, chunk: &ChunkMeta) -> Result<AviMainHeader> {
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + chunk.chunk_size);
    // TODO verify size
    Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const _) })
}

fn parse_stream_header(riff: &RiffFile, chunk: &ChunkMeta) -> Result<AviStreamHeader> {
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + chunk.chunk_size);
    // TODO verify size
    Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const _) })
}

fn parse_stream_format(riff: &RiffFile, chunk: &ChunkMeta) -> Result<BitMapInfo> {
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + chunk.chunk_size);
    // TODO verify size
    Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const _) })
}

fn find_mandatory_list<'a>(entries: &'a [Entry], list_type: FourCC) -> Result<&'a ListMeta> {
    let list = entries.iter().find_map(|e| match e {
        Entry::List(meta) if meta.list_type == list_type => Some(meta),
        _ => None,
    });
    list.ok_or_else(|| {
        Error::new(
            ErrorKind::Other,
            AviError::new(format!(
                "AVI file is missing mandatory list '{}'",
                format_fourcc(list_type)
            )),
        )
    })
}

fn find_mandatory_list_in_list<'a>(
    parent: &'a ListMeta,
    list_type: FourCC,
) -> Result<&'a ListMeta> {
    let list = parent.children.iter().find_map(|e| match e {
        Entry::List(meta) if meta.list_type == list_type => Some(meta),
        _ => None,
    });
    list.ok_or_else(|| {
        Error::new(
            ErrorKind::Other,
            AviError::new(format!(
                "List '{}' is missing mandatory list '{}'",
                format_fourcc(parent.list_type),
                format_fourcc(list_type),
            )),
        )
    })
}

fn find_mandatory_chunk<'a>(meta: &'a ListMeta, chunk_id: FourCC) -> Result<&'a ChunkMeta> {
    let chunk = meta.children.iter().find_map(|e| match e {
        Entry::Chunk(chunk) if chunk.chunk_id == chunk_id => Some(chunk),
        _ => None,
    });
    chunk.ok_or_else(|| {
        Error::new(
            ErrorKind::Other,
            AviError::new(format!(
                "List '{}' is missing mandatory chunk '{}'",
                format_fourcc(meta.list_type),
                format_fourcc(chunk_id)
            )),
        )
    })
}

#[derive(Debug)]
struct AviError {
    message: String,
}

impl AviError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl error::Error for AviError {}

impl Display for AviError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AviError: {}", &self.message)
    }
}

#[derive(Debug)]
pub enum AviEntry {
    MainHeader(AviMainHeader),
    StreamHeader(AviStreamHeader),
    BitMapInfo(BitMapInfo),
    Frame(ChunkMeta),
    Unknown(ChunkMeta),
    List(Vec<AviEntry>),
}

#[derive(Debug)]
#[repr(C)]
pub struct AviMainHeader {
    pub micro_sec_per_frame: u32,
    pub max_bytes_per_sec: u32,
    pub padding_granularity: u32,
    pub flags: u32,
    pub total_frames: u32,
    pub initial_frames: u32,
    pub streams: u32,
    pub suggested_buffer_size: u32,
    pub width: u32,
    pub height: u32,
    pub reserved: [u8; 4],
}

#[derive(Debug)]
#[repr(C)]
pub struct AviStreamHeader {
    /// Stream type. Could be `auds`, `mids`, `txts`, `vids`.
    pub fcc_type: [u8; 4],
    pub fcc_handler: [u8; 4],
    pub flags: u32,
    pub priority: u16,
    pub language: u16,
    pub initial_frames: u32,
    pub scale: u32,
    pub rate: u32,
    pub start: u32,
    pub length: u32,
    pub suggested_buffer_size: u32,
    pub quality: u32,
    pub sample_size: u32,
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

// https://docs.fileformat.com/image/dib/

#[derive(Debug)]
#[repr(C)]
pub struct BitMapInfo {
    pub header: BitMapInfoHeader,
    pub rgb: RgbQuad,
}

#[derive(Debug)]
#[repr(C)]
pub struct BitMapInfoHeader {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bit_count: u16,
    pub compression: u32,
    pub size_image: u32,
    pub x_pels_per_meter: u64,
    pub y_pels_per_meter: u64,
    pub clr_used: u32,
    pub clr_important: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct RgbQuad {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub reserved: u8,
}

/*
typedef struct _avioldindex {
   FOURCC  fcc;
   DWORD   cb;
   struct _avioldindex_entry {
      DWORD   dwChunkId;
      DWORD   dwFlags;
      DWORD   dwOffset;
      DWORD   dwSize;
  } aIndex[];
} AVIOLDINDEX;
 */

/* DML extended index
struct _aviindex_chunk {
   FOURCC fcc;
   DWORD  cb;
   WORD   wLongsPerEntry;   // size of each entry in aIndex array
   BYTE   bIndexSubType;    // future use.  must be 0
   BYTE   bIndexType;       // one of AVI_INDEX_* codes
   DWORD  nEntriesInUse;    // index of first unused member in aIndex array
   DWORD  dwChunkId;        // fcc of what is indexed
   DWORD  dwReserved[3];    // meaning differs for each index
                            // type/subtype.   0 if unused
   struct _aviindex_entry {
      DWORD adw[wLongsPerEntry];
   } aIndex[ ];
};
 */

fn format_fourcc(value: FourCC) -> String {
    match str::from_utf8(&value) {
        Ok(s) => s.to_string(),
        _ => format!("{:x?}", value),
    }
}
