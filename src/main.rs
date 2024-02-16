use std::process;

pub use colors::Lab;
pub use imager::LabImage;
pub use collager::Vec2;

use collager::Collager;
use imager::Imager;
use config::Config;

mod collager;
mod imager;
mod config;

pub mod colors;


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
        config.pixel_size,
        config.output_indices
    );

    let imager_config = imager::Config{
        image_size: config.pixel_size,
        allow_rotate: config.allow_rotate,
        allow_invert: config.allow_invert,
        depth: config.depth
    };

    let imager = Imager::new(config.directory, imager_config)
        .unwrap_or_else(|err| complain(&format!("error opening image directory: {err:?}")));

    if config.debug
    {
        imager.save("output/");
    }

    let collage = collager.collage(imager.images());

    collage.save(config.output).unwrap();
}
