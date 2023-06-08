use std::{
    process
};

use collager::Collager;
use imager::Imager;
use config::Config;

mod collager;
mod imager;
mod config;


fn complain(message: &str) -> !
{
    eprintln!("{message}");

    process::exit(1)
}

fn main()
{
    let config = Config::parse();

    let image = image::open(config.input)
        .unwrap_or_else(|err| complain(&format!("error opening image: {err:?}")));

    let collager = Collager::new(
        image.into_rgb8(),
        config.width,
        config.pixel_size
    );

    let imager = Imager::new(config.directory, config.pixel_size)
        .unwrap_or_else(|err| complain(&format!("error opening image directory: {err:?}")));

    if config.debug
    {
        imager.save("output/");
    }

    let collage = collager.collage(imager.images());

    collage.save(config.output).unwrap();
}
