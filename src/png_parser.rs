use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt};
use thiserror::Error;

use crate::decoder::decode_data;

const MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Error, Debug)]
pub enum PngError {
    #[error("invalid png header magic")]
    InvalidMagic,
    #[error("expected chunk type {expected:?}, was {was:?}")]
    UnexpectedChunkType { expected: ChunkType, was: ChunkType },
    #[error("invalid bit depth {0}")]
    UnknownBitDepth(u8),
    #[error("invalid colour type {0}")]
    UnknownColourType(u8),
    #[error("invalid combination of bit depth and colour: {bit_depth:?}, {colour_type:?}")]
    InvalidBitDepthColourCombination {
        bit_depth: BitDepth,
        colour_type: ColourType,
    },
    #[error("invalid compression method {0}")]
    UnknownCompressionMethod(u8),
    #[error("invalid filter method {0}")]
    UnknownFilterMethod(u8),
    #[error("invalid interlace method {0}")]
    UnknownInterlaceMethod(u8),
    #[error("inflate error: {0}")]
    InflateError(String),
    #[error("io error")]
    IoError(#[from] std::io::Error),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ChunkType {
    IHDR,
    PLTE,
    IDAT,
    IEND,
    Unknown([u8; 4]),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BitDepth {
    B1,
    B2,
    B4,
    B8,
    B16,
}

impl BitDepth {
    pub fn to_bytes(&self) -> usize {
        match self {
            BitDepth::B8 => 1,
            BitDepth::B16 => 2,
            otherwise => panic!("unsupported bit depth: {:?}", otherwise),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ColourType {
    Greyscale,
    Truecolour,
    IndexedColour,
    GreyscaleWithAlpha,
    TruecolourWithAlpha,
}

impl ColourType {
    pub fn channel_count(self) -> usize {
        match self {
            ColourType::Greyscale => 1,
            ColourType::Truecolour => 3,
            ColourType::IndexedColour => 1,
            ColourType::GreyscaleWithAlpha => 1,
            ColourType::TruecolourWithAlpha => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InterlaceMethod {
    None,
    Adam7,
}

pub struct PngChunk {
    length: u32,
    pub chunk_type: ChunkType,
    pub(crate) data: Vec<u8>,
    crc: [u8; 4],
}

impl std::fmt::Debug for PngChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PngChunk")
            .field("length", &self.length)
            .field("chunk_type", &self.chunk_type)
            .field("crc", &self.crc)
            .finish()
    }
}

#[derive(Debug)]
pub struct PngHeader {
    pub width: u32,
    pub height: u32,
    pub(crate) bit_depth: BitDepth,
    pub(crate) colour_type: ColourType,
    pub(crate) interlace_method: InterlaceMethod,
}

impl PngHeader {
    pub fn bit_depth(&self) -> BitDepth {
        self.bit_depth
    }

    pub fn colour_type(&self) -> ColourType {
        self.colour_type
    }
}

impl<'a> TryFrom<&'a PngChunk> for PngHeader {
    type Error = PngError;

    fn try_from(value: &'a PngChunk) -> Result<Self, Self::Error> {
        if value.chunk_type != ChunkType::IHDR {
            return Err(PngError::UnexpectedChunkType {
                expected: ChunkType::IHDR,
                was: value.chunk_type,
            });
        }

        let mut reader = Cursor::new(&value.data);

        let width = reader.read_u32::<BigEndian>()?;
        let height = reader.read_u32::<BigEndian>()?;

        let bit_depth = reader.read_u8()?;
        let colour_type = reader.read_u8()?;

        let colour_type = match colour_type {
            0 => ColourType::Greyscale,
            2 => ColourType::Truecolour,
            3 => ColourType::IndexedColour,
            4 => ColourType::GreyscaleWithAlpha,
            6 => ColourType::TruecolourWithAlpha,
            unknown => return Err(PngError::UnknownColourType(unknown)),
        };

        let bit_depth = match bit_depth {
            1 => BitDepth::B1,
            2 => BitDepth::B2,
            4 => BitDepth::B4,
            8 => BitDepth::B8,
            16 => BitDepth::B16,
            unknown => return Err(PngError::UnknownBitDepth(unknown)),
        };

        match (colour_type, bit_depth) {
            (
                ColourType::Greyscale,
                BitDepth::B1 | BitDepth::B2 | BitDepth::B4 | BitDepth::B8 | BitDepth::B16,
            )
            | (ColourType::Truecolour, BitDepth::B8 | BitDepth::B16)
            | (
                ColourType::IndexedColour,
                BitDepth::B1 | BitDepth::B2 | BitDepth::B4 | BitDepth::B8,
            )
            | (ColourType::GreyscaleWithAlpha, BitDepth::B8 | BitDepth::B16)
            | (ColourType::TruecolourWithAlpha, BitDepth::B8 | BitDepth::B16) => (),
            (colour_type, bit_depth) => {
                return Err(PngError::InvalidBitDepthColourCombination {
                    colour_type,
                    bit_depth,
                })
            }
        }

        let compression_method = reader.read_u8()?;

        if compression_method != 0 {
            return Err(PngError::UnknownCompressionMethod(compression_method));
        }

        let filter_method = reader.read_u8()?;

        if filter_method != 0 {
            return Err(PngError::UnknownFilterMethod(filter_method));
        }

        let interlace_method = reader.read_u8()?;

        let interlace_method = match interlace_method {
            0 => InterlaceMethod::None,
            1 => InterlaceMethod::Adam7,
            unknown => return Err(PngError::UnknownInterlaceMethod(unknown)),
        };

        Ok(PngHeader {
            width,
            height,
            bit_depth,
            colour_type,
            interlace_method,
        })
    }
}

#[derive(Debug)]
pub struct PngFile {
    pub chunks: Vec<PngChunk>,
}

impl PngFile {
    pub fn get_header_chunk(&self) -> &PngChunk {
        &self.chunks[0]
    }

    pub fn try_parse_header(&self) -> Result<PngHeader, PngError> {
        PngHeader::try_from(self.get_header_chunk())
    }

    pub fn from_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, PngError> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(PngError::InvalidMagic);
        }

        let mut chunks = Vec::new();

        loop {
            let chunk = parse_png_chunk(reader)?;
            let chunk_type = chunk.chunk_type;
            chunks.push(chunk);

            if chunk_type == ChunkType::IEND {
                break;
            }
        }

        Ok(PngFile { chunks })
    }

    fn image_data_chunks(&self) -> impl Iterator<Item = &PngChunk> {
        self.chunks
            .iter()
            .filter(|chunk| chunk.chunk_type == ChunkType::IDAT)
    }

    pub fn decode_data(&self) -> Result<Vec<u8>, PngError> {
        let mut buffer = Vec::new();
        self.decode_data_to(&mut buffer)?;
        Ok(buffer)
    }

    pub fn decode_data_to(&self, out: &mut Vec<u8>) -> Result<(), PngError> {
        let header = self.try_parse_header()?;
        decode_data(&header, self.image_data_chunks(), out)
    }
}

fn parse_png_chunk<R: std::io::Read>(reader: &mut R) -> Result<PngChunk, PngError> {
    let length = reader.read_u32::<BigEndian>()?;
    let mut chunk_type = [0u8; 4];
    reader.read_exact(&mut chunk_type)?;

    let chunk_type = match &chunk_type {
        b"IHDR" => ChunkType::IHDR,
        b"PLTE" => ChunkType::PLTE,
        b"IDAT" => ChunkType::IDAT,
        b"IEND" => ChunkType::IEND,
        otherwise => ChunkType::Unknown(*otherwise),
    };

    let mut data = Vec::with_capacity(length as usize);
    reader.take(length as u64).read_to_end(&mut data)?;

    let mut crc = [0u8; 4];
    reader.read_exact(&mut crc)?;

    Ok(PngChunk {
        length,
        chunk_type,
        data,
        crc,
    })
}
