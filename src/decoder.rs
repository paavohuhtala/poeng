use std::io::Write;

use inflate::InflateWriter;

use crate::png_parser::{ChunkType, PngChunk, PngError, PngHeader};

fn filter_none(
    x: u8,
    _previous: u8,
    _scanline_offset: usize,
    _previous_scanline: &[u8],
    _pixel_size: usize,
) -> u8 {
    x
}

#[inline]
fn filter_sub(
    x: u8,
    previous: u8,
    _scanline_offset: usize,
    _previous_scanline: &[u8],
    _pixel_size: usize,
) -> u8 {
    x.wrapping_add(previous)
}

#[inline]
fn filter_up(
    x: u8,
    _previous: u8,
    scanline_offset: usize,
    previous_scanline: &[u8],
    _pixel_size: usize,
) -> u8 {
    let b = previous_scanline[scanline_offset];
    x.wrapping_add(b)
}

#[inline]
fn filter_average(
    x: u8,
    previous: u8,
    scanline_offset: usize,
    previous_scanline: &[u8],
    _pixel_size: usize,
) -> u8 {
    let a = previous;
    let b = previous_scanline[scanline_offset];

    (x as i32 + ((a as i32 + b as i32) / 2)) as u8
}

#[inline]
fn filter_paeth(
    x: u8,
    previous: u8,
    scanline_offset: usize,
    previous_scanline: &[u8],
    pixel_size: usize,
) -> u8 {
    let a = previous;
    let b = previous_scanline[scanline_offset];
    let c = if scanline_offset >= pixel_size {
        previous_scanline[scanline_offset - pixel_size]
    } else {
        0
    };

    x.wrapping_add(paeth_predictor(a, b, c))
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;

    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

pub fn decode_data<'a>(
    header: &'a PngHeader,
    chunks: impl Iterator<Item = &'a PngChunk>,
    decoded_data_out: &mut Vec<u8>,
) -> Result<(), PngError> {
    let mut inflate_writer = InflateWriter::from_zlib(Vec::new());

    for chunk in chunks {
        assert_eq!(chunk.chunk_type, ChunkType::IDAT);
        inflate_writer.write_all(&chunk.data)?;
    }

    let decompressed = inflate_writer.finish()?;

    // TODO: handle 1-4 bit depth
    let bytes_per_channel = header.bit_depth.to_bytes();
    let number_of_channels = header.colour_type.channel_count();
    let bytes_per_pixel = number_of_channels * bytes_per_channel;

    let scanline_length = header.width as usize * bytes_per_pixel;
    let scanline_length_with_filter = scanline_length + 1;

    decoded_data_out.resize(scanline_length * header.height as usize, 0);

    let mut previous_scanline = vec![0u8; scanline_length];

    let input_chunks = decompressed.chunks_exact(scanline_length_with_filter);
    let output_chunks = decoded_data_out.chunks_exact_mut(scanline_length);

    for (scanline_in, scanline_out) in input_chunks.zip(output_chunks) {
        let (filter_type, scanline_in) = scanline_in.split_first().unwrap();

        let filter = match filter_type {
            0 => filter_none,
            1 => filter_sub,
            2 => filter_up,
            3 => filter_average,
            4 => filter_paeth,
            _ => panic!("Invalid filter type"),
        };

        for (scanline_offset, byte) in scanline_in.iter().copied().enumerate() {
            let previous = if scanline_offset >= bytes_per_pixel {
                scanline_out[scanline_offset - bytes_per_pixel]
            } else {
                0
            };

            let decoded = filter(
                byte,
                previous,
                scanline_offset,
                &previous_scanline,
                bytes_per_pixel,
            );

            scanline_out[scanline_offset] = decoded;
        }

        previous_scanline.copy_from_slice(scanline_out);
    }

    Ok(())
}
