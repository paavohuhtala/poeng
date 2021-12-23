use std::num::Wrapping;

use crate::png_parser::{BitDepth, PngChunk, PngError, PngHeader};

type FilterFunction =
    fn(x: u8, offset: usize, scanline_offset: usize, decoded: &mut [u8], scanline_length: usize);

fn filter_none(
    x: u8,
    offset: usize,
    scanline_offset: usize,
    decoded: &mut [u8],
    _scanline_length: usize,
) {
    decoded[offset] = x;
}

fn filter_sub(
    x: u8,
    offset: usize,
    scanline_offset: usize,
    decoded: &mut [u8],
    _scanline_length: usize,
) {
    let a = if scanline_offset > 0 {
        decoded[offset - 1]
    } else {
        0
    };

    decoded[offset] = x.wrapping_add(a);
}

fn filter_up(
    x: u8,
    offset: usize,
    scanline_offset: usize,
    decoded: &mut [u8],
    scanline_length: usize,
) {
    // offset != scanline_offset if we're past line 0
    let b = if offset > scanline_offset {
        decoded[offset - scanline_length]
    } else {
        0
    };

    decoded[offset] = x.wrapping_add(b);
}

fn filter_average(
    x: u8,
    offset: usize,
    scanline_offset: usize,
    decoded: &mut [u8],
    scanline_length: usize,
) {
    let a = if scanline_offset > 0 {
        decoded[offset - 1]
    } else {
        0
    };

    let b = if offset > scanline_offset {
        decoded[offset - scanline_length]
    } else {
        0
    };

    decoded[offset] = x.wrapping_add(a.wrapping_add(b) / 2);
}

fn filter_paeth(
    x: u8,
    offset: usize,
    scanline_offset: usize,
    decoded: &mut [u8],
    scanline_length: usize,
) {
    let a = if scanline_offset > 0 {
        decoded[offset - 1]
    } else {
        0
    };

    let b = if offset > scanline_offset {
        decoded[offset - scanline_length]
    } else {
        0
    };

    let c = if scanline_offset > 0 && offset > scanline_offset {
        decoded[offset - scanline_length - 1]
    } else {
        0
    };

    decoded[offset] = x.wrapping_add(paeth_predictor(Wrapping(a), Wrapping(b), Wrapping(c)));
}

fn paeth_predictor(a: Wrapping<u8>, b: Wrapping<u8>, c: Wrapping<u8>) -> u8 {
    fn paeth_diff(a: Wrapping<u8>, b: Wrapping<u8>) -> Wrapping<u8> {
        Wrapping(((a.0 as i32 - b.0 as i32).abs() % 256) as u8)
    }

    let p = a + b - c;
    let pa = paeth_diff(p, a);
    let pb = paeth_diff(p, b);
    let pc = paeth_diff(p, c);

    if pa <= pb && pa <= pc {
        a.0
    } else if pb <= pc {
        b.0
    } else {
        c.0
    }
}

pub fn decode_data(header: &PngHeader, data: &PngChunk) -> Result<Vec<u8>, PngError> {
    assert_eq!(header.bit_depth, BitDepth::B8, "bit depth must be 8");

    let decompressed = inflate::inflate_bytes_zlib(&data.data).map_err(PngError::InflateError)?;

    // todo handle non 8-bit images
    let bytes_per_pixel = match header.colour_type {
        crate::png_parser::ColourType::Greyscale => 1,
        crate::png_parser::ColourType::Truecolour => 3,
        crate::png_parser::ColourType::IndexedColour => 1,
        crate::png_parser::ColourType::GreyscaleWithAlpha => 1,
        crate::png_parser::ColourType::TruecolourWithAlpha => 4,
    };

    let scanline_length = header.width as usize * bytes_per_pixel;
    let scanline_length_with_filter = scanline_length + 1;

    let mut decoded_data = vec![0u8; scanline_length * header.height as usize];
    let mut decoded_offset = 0;

    for scanline in decompressed.chunks_exact(scanline_length_with_filter) {
        let filter_type = scanline[0];

        let filter = match filter_type {
            0 => {
                println!("filter type none");
                filter_none
            }
            1 => {
                println!("filter type sub");
                filter_sub
            }
            2 => {
                println!("filter type up");
                filter_up
            }
            3 => {
                println!("filter type average");
                filter_average
            }
            4 => {
                println!("filter type paeth");
                filter_paeth
            }
            _ => panic!("Invalid filter type"),
        };

        for (scanline_offset, byte) in scanline[1..].iter().copied().enumerate() {
            filter(
                byte,
                decoded_offset,
                scanline_offset,
                &mut decoded_data,
                scanline_length,
            );
            decoded_offset += 1;
        }
    }

    assert_eq!(decoded_offset, decoded_data.len());

    Ok(decoded_data)
}
