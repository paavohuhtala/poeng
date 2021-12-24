use std::{fs::File, io::Write};

use image::{RgbImage, RgbaImage};
use poeng::{self, png_parser::PngFile};

pub fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let file = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("./FL.png"));
    let mut file = File::open(file.as_str()).unwrap();

    let png = PngFile::from_reader(&mut file).unwrap();
    println!("{:?}", png);

    let header = png.try_parse_header().unwrap();
    println!("{:?}", header);

    let decoded = png.decode_data().unwrap();

    File::create("out.bin")
        .unwrap()
        .write_all(&decoded)
        .unwrap();

    match header.colour_type() {
        poeng::png_parser::ColourType::Truecolour => {
            RgbImage::from_raw(header.width, header.height, decoded)
                .unwrap()
                .save("roundtrip.png")
                .unwrap();
        }
        poeng::png_parser::ColourType::TruecolourWithAlpha => {
            RgbaImage::from_raw(header.width, header.height, decoded)
                .unwrap()
                .save("roundtrip.png")
                .unwrap();
        }
        _ => panic!("unsupported colour type"),
    }
}
