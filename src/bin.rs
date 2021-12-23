use std::{fs::File, io::Write};

use image::RgbImage;
use poeng::{self, decoder::decode_data, png_parser::ChunkType};

pub fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let file = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("./PNG-Gradient.png"));
    let mut file = File::open(file.as_str()).unwrap();

    let png = poeng::png_parser::parse_png(&mut file).unwrap();
    println!("{:?}", png);

    let header = png.try_parse_header().unwrap();
    println!("{:?}", header);

    let data = png
        .chunks
        .iter()
        .find(|chunk| chunk.chunk_type == ChunkType::IDAT)
        .unwrap();

    let decoded = decode_data(&header, data).unwrap();

    File::create("out.bin")
        .unwrap()
        .write_all(&decoded)
        .unwrap();

    let buffer = RgbImage::from_raw(header.width, header.height, decoded).unwrap();
    buffer.save("roundtrip.png").unwrap();
}
