use std::{
    fs,
    thread,
    path::PathBuf,
    sync::Arc,
    ops::ControlFlow
};

use image::{
    Rgb,
    RgbImage,
    ImageBuffer,
    imageops::{self, FilterType}
};

use crate::{
    Lab,
    LabImage,
    imager::{LabImagesContainer, ImagesContainer}
};


const SQRT_DISTANCE: bool = false;

pub struct Vec2
{
    pub x: u32,
    pub y: u32
}

pub struct Collager
{
    image: LabImage,
    width: u32,
    height: u32,
    pixel_size: u32,
    output_indices: Option<PathBuf>
}

impl Collager
{
    pub fn new(
        image: RgbImage,
        width: u32,
        pixel_size: u32,
        output_indices: Option<PathBuf>
    ) -> Self
    {
        let total_width = width * pixel_size;
        let width_scale = total_width as f64 / image.width() as f64;

        let total_height = (image.height() as f64 * width_scale).ceil() as u32;

        let filter_type = FilterType::CatmullRom;

        let image: LabImage = imageops::resize(&image, total_width, total_height, filter_type)
            .into();

        let height = total_height / pixel_size;

        Self{image, width, height, pixel_size, output_indices}
    }

    pub fn collage(&self, images: Arc<ImagesContainer>) -> RgbImage
    {
        let lab_images: LabImagesContainer = images.iter().cloned().map(|pair|
        {
            LabImage::from(pair.image)
        }).collect();

        let indices = self.best_indices(Arc::new(lab_images));

        self.construct_from_indices(indices.into_iter(), &images)
    }

    fn positions_iter(&self) -> impl Iterator<Item=Vec2> + '_
    {
        (0..self.height).flat_map(move |y|
        {
            (0..self.width).map(move |x|
            {
                Vec2{x: x * self.pixel_size, y: y * self.pixel_size}
            })
        })
    }

    fn construct_from_indices(
        &self,
        indices: impl Iterator<Item=usize> + Clone,
        images: &ImagesContainer
    ) -> RgbImage
    {
        if let Some(path) = self.output_indices.as_ref()
        {
            let mut s = String::new();

            let mut indices = indices.clone();
            for y in 0..self.height
            {
                for x in 0..self.width
                {
                    let index = indices.next()
                        .expect("width * height must be equal to amount of indices");

                    s += &images[index].name;

                    // if not last
                    if x != (self.width - 1)
                    {
                        s.push(' ');
                    }
                }

                // if not last
                if y != (self.height - 1)
                {
                    s.push('\n');
                }
            }

            // if it crashes then its over
            fs::write(path, s).unwrap();
        }

        let mut image =
        {
            let pixel = Rgb::<u8>::from([0, 0, 0]);

            let width = self.width * self.pixel_size;
            let height = self.height * self.pixel_size;

            ImageBuffer::from_pixel(width, height, pixel)
        };

        self.positions_iter().zip(indices).for_each(|(position, index)|
        {
            let mut copy_pixel = |x, y|
            {
                let pixel = images[index].image.get_pixel(x, y);

                image.put_pixel(position.x + x, position.y + y, *pixel);
            };

            for y in 0..self.pixel_size
            {
                for x in 0..self.pixel_size
                {
                    copy_pixel(x, y);
                }
            }
        });

        image
    }

    fn best_indices(&self, images: Arc<LabImagesContainer>) -> Vec<usize>
    {
        let handles = self.positions_iter().map(move |position|
        {
            let image = self.image.clone();
            let images = images.clone();

            let size = Vec2{x: self.pixel_size, y: self.pixel_size};

            thread::spawn(move ||
            {
                Self::best_fit_index_assoc(
                    &image,
                    images.iter(),
                    position,
                    size
                )
            })
        }).collect::<Vec<_>>();

        handles.into_iter().map(|handle| handle.join().unwrap()).collect()
    }

    #[allow(dead_code)]
    fn best_fit_index<'a>(
        &self,
        images: impl Iterator<Item=&'a LabImage>,
        position: Vec2
    ) -> usize
    {
        Self::best_fit_index_assoc(
            &self.image,
            images,
            position,
            Vec2{x: self.pixel_size, y: self.pixel_size}
        )
    }

    #[allow(dead_code)]
    fn best_fit_index_assoc<'a>(
        image: &LabImage,
        images: impl Iterator<Item=&'a LabImage>,
        position: Vec2,
        size: Vec2
    ) -> usize
    {
        let subimage = image.subimage_pixels(position, size);

        Self::best_fit_index_associated(subimage.iter().copied(), images)
    }

    fn best_fit_index_associated<'a, I>(
        subimage: I,
        images: impl Iterator<Item=&'a LabImage>
    ) -> usize
    where
        I: Iterator<Item=Lab> + Clone
    {
        struct BestFit
        {
            index: usize,
            error: f32
        }

        let images = images.enumerate();

        let mut best_fit = BestFit{
            index: 0,
            error: f32::INFINITY
        };

        images.for_each(|(index, image)|
        {
            let error = Self::pixels_error_early_exit(
                subimage.clone(),
                image.pixels(),
                best_fit.error
            );

            if let Some(error) = error
            {
                if error < best_fit.error
                {
                    best_fit = BestFit{index, error};
                }
            }
        });

        best_fit.index
    }

    fn pixels_error_early_exit<'a, A, B>(a: A, b: B, min_bound: f32) -> Option<f32>
    where
        A: Iterator<Item=Lab>,
        B: Iterator<Item=Lab>
    {
        let error = a.zip(b).map(|(a, b)|
        {
            if SQRT_DISTANCE
            {
                a.distance(b).sqrt()
            } else
            {
                a.distance(b)
            }
        }).try_fold(0.0, |mut acc, distance|
        {
            acc += distance;

            if acc >= min_bound
            {
                ControlFlow::Break(())
            } else
            {
                ControlFlow::Continue(acc)
            }
        });

        match error
        {
            ControlFlow::Continue(x) => Some(x),
            ControlFlow::Break(_) => None
        }
    }
}
