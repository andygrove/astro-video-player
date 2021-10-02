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

use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};
use std::str;
use std::{error, fmt};

use riff_io::{ChunkMeta, Entry, FourCC, ListMeta, RiffFile};

// use https://www.rapidtables.com/convert/number/ascii-to-hex.html

const FOURCC_AVIH: FourCC = [0x61, 0x76, 0x69, 0x68];
//const FOURCC_JUNK: FourCC = [0x4a, 0x55, 0x4e, 0x4b];
const FOURCC_HDRL: FourCC = [0x68, 0x64, 0x72, 0x6c];
const FOURCC_STRH: FourCC = [0x73, 0x74, 0x72, 0x68];
const FOURCC_STRF: FourCC = [0x73, 0x74, 0x72, 0x66];
const FOURCC_INDX: FourCC = [0x69, 0x6e, 0x64, 0x78];
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

        /*
                LIST 'hdrl'
          CHUNK 'avih' offset=32 size=56
          LIST 'strl'
            CHUNK 'strh' offset=108 size=56
            CHUNK 'strf' offset=172 size=1064
            CHUNK 'indx' offset=1244 size=32248
          LIST 'odml'
            CHUNK 'dmlh' offset=33512 size=248
        CHUNK 'JUNK' offset=33768 size=12
        LIST 'movi'
          CHUNK 'ix00' offset=33800 size=32248
          CHUNK '00db' offset=66056 size=3818112
          CHUNK 'JUNK' offset=3884176 size=368
          ...
          CHUNK '00db' offset=164261384 size=3818112
          CHUNK 'JUNK' offset=168079504 size=368
        CHUNK 'idx1' offset=168079880 size=1528
                 */

        // main header
        let hdrl = find_mandatory_list(&entries, FOURCC_HDRL)?;
        let chunk = find_mandatory_chunk(hdrl, FOURCC_AVIH)?;
        let main_header = parse_main_header(&riff, chunk)?;

        // get first stream header and format (only one stream is currently supported)
        let strl = find_mandatory_list_in_list(hdrl, FOURCC_STRL)?;
        let strh = find_mandatory_chunk(strl, FOURCC_STRH)?;
        let strf = find_mandatory_chunk(strl, FOURCC_STRF)?;
        let _indx = find_mandatory_chunk(strl, FOURCC_INDX)?;
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
    assert!(chunk.data_size >= 44);
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + chunk.chunk_size);
    Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const _) })
}

fn parse_stream_header(riff: &RiffFile, chunk: &ChunkMeta) -> Result<AviStreamHeader> {
    assert!(chunk.data_size >= 56);
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + chunk.chunk_size);
    Ok(unsafe { std::ptr::read(bytes.as_ptr() as *const _) })
}

fn parse_stream_format(riff: &RiffFile, chunk: &ChunkMeta) -> Result<BitMapInfo> {
    assert!(chunk.data_size >= 48);
    let bytes = riff.read_bytes(chunk.data_offset..chunk.data_offset + 48);

    let header: BitMapInfoHeader = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };

    // https://docs.microsoft.com/en-us/previous-versions/dd183376(v=vs.85)
    match header.bit_count {
        0 => {
            // The number of bits-per-pixel is specified or is implied by the JPEG or PNG format.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("JPG and PNG are not supported".to_string()),
            ))
        }
        1 => {
            // The bitmap is monochrome, and the bmiColors member of BITMAPINFO contains two
            // entries. Each bit in the bitmap array represents a pixel. If the bit is clear,
            // the pixel is displayed with the color of the first entry in the bmiColors table;
            // if the bit is set, the pixel has the color of the second entry in the table.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("Monochrome images are not supported".to_string()),
            ))
        }
        4 => {
            // The bitmap has a maximum of 16 colors, and the bmiColors member of BITMAPINFO
            // contains up to 16 entries. Each pixel in the bitmap is represented by a 4-bit
            // index into the color table. For example, if the first byte in the bitmap is 0x1F,
            // the byte represents two pixels. The first pixel contains the color in the second
            // table entry, and the second pixel contains the color in the sixteenth table entry.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("Unsupported bit_count".to_string()),
            ))
        }
        8 => {
            // The bitmap has a maximum of 256 colors, and the bmiColors member of BITMAPINFO
            // contains up to 256 entries. In this case, each byte in the array represents a
            // single pixel.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("Unsupported bit_count".to_string()),
            ))
        }
        16 => {
            // The bitmap has a maximum of 2^16 colors. If the biCompression member of the
            // BITMAPINFOHEADER is BI_RGB, the bmiColors member of BITMAPINFO is NULL. Each
            // WORD in the bitmap array represents a single pixel. The relative intensities of
            // red, green, and blue are represented with five bits for each color component.
            // The value for blue is in the least significant five bits, followed by five bits
            // each for green and red. The most significant bit is not used. The bmiColors
            // color table is used for optimizing colors used on palette-based devices, and
            // must contain the number of entries specified by the biClrUsed member of the
            // BITMAPINFOHEADER.
            //
            // If the biCompression member of the BITMAPINFOHEADER is BI_BITFIELDS, the
            // bmiColors member contains three DWORD color masks that specify the red,
            // green, and blue components, respectively, of each pixel. Each WORD in the
            // bitmap array represents a single pixel.
            //
            // When the biCompression member is BI_BITFIELDS, bits set in each DWORD mask
            // must be contiguous and should not overlap the bits of another mask. All the
            // bits in the pixel do not have to be used.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("Unsupported bit_count".to_string()),
            ))
        }
        24 => {
            // The bitmap has a maximum of 2^24 colors, and the bmiColors member of BITMAPINFO
            // is NULL. Each 3-byte triplet in the bitmap array represents the relative intensities
            // of blue, green, and red, respectively, for a pixel. The bmiColors color table is
            // used for optimizing colors used on palette-based devices, and must contain the
            // number of entries specified by the biClrUsed member of the BITMAPINFOHEADER.
            Ok(BitMapInfo {
                header,
                color_coding: ColorCoding::BGR,
                rgb: vec![],
            })
        }
        32 => {
            // The bitmap has a maximum of 2^32 colors. If the biCompression member of the
            // BITMAPINFOHEADER is BI_RGB, the bmiColors member of BITMAPINFO is NULL. Each
            // DWORD in the bitmap array represents the relative intensities of blue, green,
            // and red for a pixel. The value for blue is in the least significant 8 bits,
            // followed by 8 bits each for green and red. The high byte in each DWORD is not
            // used. The bmiColors color table is used for optimizing colors used on
            // palette-based devices, and must contain the number of entries specified by the
            // biClrUsed member of the BITMAPINFOHEADER.
            //
            // If the biCompression member of the BITMAPINFOHEADER is BI_BITFIELDS, the
            // bmiColors member contains three DWORD color masks that specify the red, green,
            // and blue components, respectively, of each pixel. Each DWORD in the bitmap
            // array represents a single pixel.
            //
            // When the biCompression member is BI_BITFIELDS, bits set in each DWORD mask must
            // be contiguous and should not overlap the bits of another mask. All the bits in
            // the pixel do not need to be used.
            Err(Error::new(
                ErrorKind::Other,
                AviError::new("Unsupported bit_count".to_string()),
            ))
        }
        _ => Err(Error::new(
            ErrorKind::Other,
            AviError::new("Invalid bit_count".to_string()),
        )),
    }
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
    pub color_coding: ColorCoding,
    pub rgb: Vec<RgbQuad>,
}

#[derive(Debug)]
pub enum ColorCoding {
    BGR,
}

#[derive(Debug)]
#[repr(C)]
pub struct BitMapInfoHeader {
    /// Specifies the number of bytes required by the structure. This value does not include the
    /// size of the color table or the size of the color masks, if they are appended to the
    /// end of structure
    pub size: u32,
    /// Specifies the width of the bitmap, in pixels
    pub width: i32,
    /// Specifies the height of the bitmap, in pixels.
    /// For uncompressed RGB bitmaps, if biHeight is positive, the bitmap is a bottom-up DIB
    /// with the origin at the lower left corner. If biHeight is negative, the bitmap is a
    /// top-down DIB with the origin at the upper left corner.
    /// For YUV bitmaps, the bitmap is always top-down, regardless of the sign of biHeight. Decoders
    /// should offer YUV formats with positive biHeight, but for backward compatibility they
    /// should accept YUV formats with either positive or negative biHeight.
    /// For compressed formats, biHeight must be positive, regardless of image orientation.
    pub height: i32,
    /// Specifies the number of planes for the target device. This value must be set to 1
    pub planes: u16,
    /// Specifies the number of bits per pixel (bpp). For uncompressed formats, this value is
    /// the average number of bits per pixel. For compressed formats, this value is the implied
    /// bit depth of the uncompressed image, after the image has been decoded.
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
