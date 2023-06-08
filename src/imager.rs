use std::{
    fs,
    io,
    path::{Path, PathBuf}
};

use image::{
    RgbImage,
    DynamicImage,
    imageops::FilterType,
    error::ImageError
};


#[allow(dead_code)]
#[derive(Debug)]
pub struct Error
{
    filename: Option<PathBuf>,
    error: ImageError
}

impl Error
{
    pub fn new<P: AsRef<Path>>(filename: P, error: ImageError) -> Self
    {
        Self{
            filename: Some(filename.as_ref().to_owned()),
            error
        }
    }
}

impl From<io::Error> for Error
{
    fn from(value: io::Error) -> Self
    {
        Self{filename: None, error: value.into()}
    }
}

pub struct Imager
{
    images: Box<[RgbImage]>
}

impl Imager
{
    pub fn new<P: AsRef<Path>>(directory: P, image_size: u32) -> Result<Self, Error>
    {
        let images = Self::create_images(directory.as_ref(), image_size)?;

        Ok(Self{images})
    }

    pub fn images(&self) -> &[RgbImage]
    {
        &self.images
    }

    fn create_images(directory: &Path, image_size: u32) -> Result<Box<[RgbImage]>, Error>
    {
        let images = directory.read_dir()?.map(|image_file|
        {
            let image_file = image_file?;
            let image_path = image_file.path();

            let image = image::open(&image_path).map_err(|err| Error::new(image_path, err))?;

            let image = Self::resize_image(image, image_size);

            Ok(image.into_rgb8())
        }).collect::<Result<Vec<_>, Error>>()?;

        Ok(images.into_boxed_slice())
    }

    fn resize_image(image: DynamicImage, image_size: u32) -> DynamicImage
    {
        let filter_type = FilterType::CatmullRom;

        let resized = image.resize_to_fill(image_size, image_size, filter_type);

        resized
    }

    pub fn save<P: AsRef<Path>>(&self, output_directory: P)
    {
        if !output_directory.as_ref().is_dir()
        {
            // i dont care (i do (no))
            fs::create_dir(output_directory.as_ref()).unwrap();
        }

        self.images.iter().enumerate().for_each(|(index, image)|
        {
            let image_name = format!("{index}.png");
            let image_path = output_directory.as_ref().join(image_name);

            // if it crashes i dont care
            image.save(image_path).unwrap();
        })
    }
}