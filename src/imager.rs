use std::{
    fs,
    io,
    thread,
    time::Duration,
    sync::Arc,
    borrow::Borrow,
    path::{Path, PathBuf}
};

use image::{
    Rgba,
    Pixel,
    RgbImage,
    RgbaImage,
    Rgba32FImage,
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

    pub fn error(&self) -> &ImageError
    {
        &self.error
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
    images: Arc<[RgbImage]>
}

impl Imager
{
    pub fn new<P: AsRef<Path>>(directory: P, config: Config) -> Result<Self, Error>
    {
        let images = Arc::from(Self::create_images(directory.as_ref(), config)?);

        Ok(Self{images})
    }

    pub fn images(&self) -> Arc<[RgbImage]>
    {
        self.images.clone()
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
    ) -> Vec<Rgba32FImage>
    {
        // pre convert to f32 for faster combining
        let original_transparent_images = original_transparent_images.iter().map(|image|
        {
            image.convert()
        }).collect::<Vec<_>>();

        if depth > 1
        {
            let mut previous_transparent_images = original_transparent_images.to_vec();

            let mut output_images = previous_transparent_images.clone();

            for _ in 0..(depth - 1)
            {
                let mut this_transparents = Vec::new();

                for transparent_image in previous_transparent_images.clone()
                {
                    for original_transparent in original_transparent_images.iter()
                    {
                        let combined = Self::combine_images_f32(
                            transparent_image.clone(),
                            original_transparent
                        );

                        this_transparents.push(combined);
                    }
                }

                output_images.extend(this_transparents.iter().cloned());

                previous_transparent_images = this_transparents;
            }

            output_images
        } else
        {
            original_transparent_images.to_vec()
        }
    }

    fn combine_images<O>(
        mut back: RgbaImage,
        other: O
    ) -> RgbaImage
    where
        O: Borrow<Rgba32FImage>
    {
        let to_f32 = |value| value as f32 / u8::MAX as f32;
        let from_f32 = |value| (value * u8::MAX as f32) as u8;

        back.pixels_mut().zip(other.borrow().pixels()).for_each(|(pixel, other_pixel)|
        {
            let blended = {
                let mut pixel: Rgba<f32> = Self::convert_pixel(*pixel, to_f32);

                pixel.blend(other_pixel);

                Self::convert_pixel(pixel, from_f32)
            };

            *pixel = blended;
        });

        back
    }

    fn combine_images_f32<O>(
        mut back: Rgba32FImage,
        other: O
    ) -> Rgba32FImage
    where
        O: Borrow<Rgba32FImage>
    {
        back.pixels_mut().zip(other.borrow().pixels()).for_each(|(pixel, other_pixel)|
        {
            pixel.blend(other_pixel);
        });

        back
    }

    fn convert_pixel<O, P, F>(pixel: Rgba<P>, mut f: F) -> Rgba<O>
    where
        F: FnMut(P) -> O
    {
        let Rgba([r, g, b, a]) = pixel;

        Rgba::from(
            [f(r), f(g), f(b), f(a)]
        )
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
        let image_handles = directory.read_dir()?.filter(|image_file|
        {
            image_file.as_ref().map(|image_file|
            {
                let is_file = image_file.file_type().ok().map(|file_type| file_type.is_file())
                .unwrap_or(false);

                is_file
            }).unwrap_or(true)
        }).map(|image_file| -> Result<_, Error>
        {
            Ok(image_file?.path())
        }).map(|image_path|
        {
            thread::spawn(move ||
            {
                let image_path = image_path?;

                let image = loop
                {
                    let image = image::open(&image_path).map_err(|err|
                    {
                        Error::new(image_path.clone(), err)
                    });

                    let is_recoverable = |err: &Error|
                    {
                        let image_error = err.error();

                        match image_error
                        {
                            ImageError::IoError(io_error) =>
                            {
                                io_error.raw_os_error().map(|code| code == 24).unwrap_or(false)
                            },
                            _ => false
                        }
                    };

                    let image = match image
                    {
                        Ok(x) => x,
                        Err(err) if is_recoverable(&err) =>
                        {
                            thread::sleep(Duration::from_millis(50));
                            continue;
                        },
                        x => return x
                    };

                    break Ok::<_, Error>(image);
                }?;

                let image = Self::resize_image(image, image_size);

                Ok(image)
            })
        }).collect::<Vec<_>>();

        let images = image_handles.into_iter().map(|handle|
        {
            handle.join().unwrap()
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