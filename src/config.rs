use std::{
    fmt::Display
};

use argparse::{ArgumentParser, StoreTrue, Store};


pub struct Config
{
    pub debug: bool,
    pub pixel_size: u32,
    pub allow_rotate: bool,
    pub allow_invert: bool,
    pub width: u32,
    pub output: String,
    pub directory: String,
    pub input: String
}

impl Config
{
    pub fn parse() -> Self
    {
        let mut config = Self::default();

        let s_description = Self::tell_default("small image size", config.pixel_size);
        let w_description = Self::tell_default(
           "amount of small images as width",
            config.width
        );
        let o_description = Self::tell_default("output image name", &config.output);

        {
            let mut parser = ArgumentParser::new();

            parser.refer(&mut config.debug)
                .add_option(&["-d", "--debug"], StoreTrue, "enable debug");

            parser.refer(&mut config.allow_rotate)
                .add_option(&["-r", "--rotate"], StoreTrue, "allow rotating the images");

            parser.refer(&mut config.allow_invert)
                .add_option(&["-I", "--invert"], StoreTrue, "allow inverting the images");

            parser.refer(&mut config.pixel_size)
                .add_option(&["-s", "--size"], Store, &s_description);

            parser.refer(&mut config.width)
                .add_option(&["-w", "--width"], Store, &w_description);

            parser.refer(&mut config.output)
                .add_option(&["-o", "--output"], Store, &o_description);

            parser.refer(&mut config.directory)
                .add_option(&["-d", "--directory"], Store, "directory of images to use as collage")
                .add_argument("directory", Store, "directory of images to use as collage")
                .required();

            parser.refer(&mut config.input)
                .add_option(&["-i", "--input"], Store, "input image to collage")
                .add_argument("input", Store, "input image to collage")
                .required();

            parser.parse_args_or_exit();
        }

        config
    }

    fn tell_default<T: Display>(text: &str, value: T) -> String
    {
        format!("{text} (default {value})")
    }
}

impl Default for Config
{
    fn default() -> Self
    {
        Self{
            debug: false,
            pixel_size: 16,
            allow_rotate: false,
            allow_invert: false,
            width: 16,
            output: "output.png".to_owned(),
            directory: String::new(),
            input: String::new()
        }
    }
}