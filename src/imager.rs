use std::{
    fs,
    io,
    borrow::Borrow,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf}
};

use image::{
    Rgba,
    Pixel,
    RgbImage,
    RgbaImage,
    ImageBuffer,
    DynamicImage,
    buffer::ConvertBuffer,
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

pub struct Config
{
    pub image_size: u32,
    pub allow_rotate: bool,
    pub allow_invert: bool,
    pub depth: u32
}

pub struct Imager
{
    images: Box<[RgbImage]>
}

impl Imager
{
    pub fn new<P: AsRef<Path>>(directory: P, config: Config) -> Result<Self, Error>
    {
        let images = Self::create_images(directory.as_ref(), config)?;

        Ok(Self{images})
    }

    pub fn images(&self) -> &[RgbImage]
    {
        &self.images
    }

    fn create_images(directory: &Path, config: Config) -> Result<Box<[RgbImage]>, Error>
    {
        if config.depth == 0
        {
            Self::created_unpermuted_images(directory, config)
        } else
        {
            Self::created_permuted_images(directory, config)
        }
    }

    fn created_unpermuted_images(
        directory: &Path,
        config: Config
    ) -> Result<Box<[RgbImage]>, Error>
    {
        Self::create_mapped_images(directory, config, |image| image.into_rgb8())
            .map(|images| images.into_boxed_slice())
    }

    fn created_permuted_images(directory: &Path, config: Config) -> Result<Box<[RgbImage]>, Error>
    {
        let depth = config.depth;
        let images = Self::create_mapped_images(directory, config, |image| image.into_rgba8())?;

        let (transparent_images, solid_images): (Vec<_>, Vec<_>) =
            images.into_iter().partition(|image|
            {
                let contains_transparency = image.pixels().any(|pixel|
                {
                    let Rgba([_r, _g, _b, a]) = pixel;

                    *a != u8::MAX
                });

                contains_transparency
            });

        let transparent_images = Self::recombine_transparents(&transparent_images, depth);

        let mut permuted_images = Vec::new();

        for solid_image in solid_images.iter()
        {
            for transparent_image in transparent_images.iter()
            {
                let permutation = Self::combine_images(solid_image.clone(), transparent_image);

                permuted_images.push(permutation);
            }
        }

        permuted_images.extend(solid_images.into_iter());

        let images = permuted_images.into_iter().map(|image| image.convert()).collect::<Vec<_>>();

        Ok(images.into_boxed_slice())
    }

    fn recombine_transparents(
        original_transparent_images: &[RgbaImage],
        depth: u32
    ) -> Vec<RgbaImage>
    {
        let mut transparent_images = original_transparent_images.to_vec();

        if depth > 1
        {
            for _ in 0..(depth - 1)
            {
                let transparent_images_iter =
                    transparent_images.iter().cloned().collect::<Vec<_>>();

                for transparent_image in transparent_images_iter
                {
                    for original_transparent in original_transparent_images.iter()
                    {
                        let combined =
                            Self::combine_images(transparent_image.clone(), original_transparent);

                        transparent_images.push(combined);
                    }
                }
            }
        }

        transparent_images
    }

    fn combine_images<P, Container, Other>(
        mut back: ImageBuffer<P, Container>,
        other: Other
    ) -> ImageBuffer<P, Container>
    where
        P: Pixel,
        Container: Deref<Target=[P::Subpixel]> + DerefMut,
        Other: Borrow<ImageBuffer<P, Container>>
    {
        back.pixels_mut().zip(other.borrow().pixels()).for_each(|(pixel, other_pixel)|
        {
            pixel.blend(other_pixel);
        });

        back
    }

    fn create_mapped_images<T, F>(
        directory: &Path,
        config: Config,
        f: F
    ) -> Result<Vec<T>, Error>
    where
        F: FnMut(DynamicImage) -> T
    {
        Self::create_dynamic_images(directory, config).map(|images|
        {
            images.into_iter().map(f).collect()
        })
    }

    fn create_dynamic_images(
        directory: &Path,
        config: Config
    ) -> Result<Vec<DynamicImage>, Error>
    {
        let mut images = Self::folder_images(directory, config.image_size)?;

        if config.allow_rotate
        {
            let mut rotated = {
                let images = images.iter();

                let rotated90 = images.clone().map(|image|
                {
                    image.rotate90()
                });

                let rotated180 = images.clone().map(|image|
                {
                    image.rotate180()
                });

                let rotated270 = images.map(|image|
                {
                    image.rotate270()
                });

                rotated90.chain(rotated180).chain(rotated270).collect::<Vec<_>>()
            };

            images.append(&mut rotated);
        }

        if config.allow_invert
        {
            let mut inverted = images.iter().cloned().map(|mut image|
            {
                image.invert();

                image
            }).collect::<Vec<_>>();

            images.append(&mut inverted);
        }

        Ok(images)
    }

    fn folder_images(directory: &Path, image_size: u32) -> Result<Vec<DynamicImage>, Error>
    {
        let images = directory.read_dir()?.filter(|image_file|
        {
            image_file.as_ref().map(|image_file|
            {
                let is_file = image_file.file_type().ok().map(|file_type| file_type.is_file())
                    .unwrap_or(false);

                is_file
            }).unwrap_or(true)
        }).map(|image_file|
        {
            let image_path = image_file?.path();

            let image = image::open(&image_path).map_err(|err| Error::new(image_path, err))?;

            let image = Self::resize_image(image, image_size);

            Ok(image)
        }).collect::<Result<Vec<_>, Error>>()?;

        Ok(images)
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