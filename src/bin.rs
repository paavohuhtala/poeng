use std::fs::File;

use poeng::{self, decoder::decode_data, png_parser::PngHeader};

pub fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let mut file = File::open(&args[1]).unwrap();

    let png = poeng::png_parser::parse_png(&mut file).unwrap();
    println!("{:?}", png);

    let header = png.try_parse_header().unwrap();
    println!("{:?}", header);

    let data = &png.chunks[1];

    let decoded = decode_data(&header, &data).unwrap();

    println!("{:?}", &decoded[0..3])
}
