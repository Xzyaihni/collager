use std::{
    fs,
    path::Path
};

use image::{
    RgbImage,
    DynamicImage,
    imageops::FilterType,
    error::ImageResult
};


pub struct Imager
{
    images: Box<[RgbImage]>
}

impl Imager
{
    pub fn new<P: AsRef<Path>>(directory: P, image_size: u32) -> ImageResult<Self>
    {
        let images = Self::create_images(directory.as_ref(), image_size)?;

        Ok(Self{images})
    }

    pub fn images(&self) -> &[RgbImage]
    {
        &self.images
    }

    fn create_images(directory: &Path, image_size: u32) -> ImageResult<Box<[RgbImage]>>
    {
        let images = directory.read_dir()?.map(|image_file|
        {
            let image_file = image_file?;

            let image = image::open(image_file.path())?;

            let image = Self::resize_image(image, image_size);

            Ok(image.into_rgb8())
        }).collect::<ImageResult<Vec<_>>>()?;

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