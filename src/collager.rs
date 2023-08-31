use std::{
    fs,
    thread,
    path::PathBuf,
    borrow::Borrow,
    sync::Arc,
    ops::{Deref, ControlFlow}
};

use image::{
    Rgb,
    RgbImage,
    SubImage,
    ImageBuffer,
    GenericImageView,
    imageops::{self, FilterType}
};

use crate::imager::ImagesContainer;


struct Pos2d
{
    x: u32,
    y: u32
}

impl Pos2d
{
    pub fn new(x: u32, y: u32) -> Self
    {
        Self{x, y}
    }
}

pub struct Collager
{
    image: RgbImage,
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

        let image = imageops::resize(&image, total_width, total_height, filter_type);

        let height = total_height / pixel_size;

        Self{image, width, height, pixel_size, output_indices}
    }

    pub fn collage(&self, images: Arc<ImagesContainer>) -> RgbImage
    {
        let indices = self.best_indices(images.clone());

        self.construct_from_indices(indices.into_iter(), &images)
    }

    fn positions_iter(&self) -> impl Iterator<Item=Pos2d> + '_
    {
        (0..self.height).flat_map(move |y|
        {
            (0..self.width).map(move |x| Pos2d::new(x * self.pixel_size, y * self.pixel_size))
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
            for _y in 0..self.height
            {
                for _x in 0..self.width
                {
                    let index = indices.next()
                        .expect("width * height must be equal to amount of indices");

                    s += &images[index].name;
                }

                s.push('\n');
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

    fn best_indices(&self, images: Arc<ImagesContainer>) -> Vec<usize>
    {
        let handles = self.positions_iter().map(move |position|
        {
            let subimage = Arc::new(self.subimage(position).to_image());
            let images = images.clone();

            thread::spawn(move ||
            {
                Self::best_fit_index_associated(
                    subimage,
                    images.iter().map(|img| &img.image)
                )
            })
        }).collect::<Vec<_>>();

        handles.into_iter().map(|handle| handle.join().unwrap()).collect()
    }

    #[allow(dead_code)]
    fn best_fit_index(&self, images: &[RgbImage], position: Pos2d) -> usize
    {
        let subimage = self.subimage(position).to_image();

        Self::best_fit_index_associated(subimage, images.iter())
    }

    fn best_fit_index_associated<I, Container, InnerImage>(
        subimage: I,
        images: impl Iterator<Item=InnerImage>
    ) -> usize
    where
        Container: Deref<Target=[u8]>,
        I: Borrow<ImageBuffer<Rgb<u8>, Container>>,
        InnerImage: Borrow<RgbImage>
    {
        let main_pixels = subimage.borrow().pixels();

        struct BestFit
        {
            index: usize,
            error: f64
        }

        let mut images = images.enumerate();

        let mut best_fit = BestFit{
            index: 0,
            error: Self::pixels_error(
                main_pixels.clone(),
                images.next().expect("images must not be empty").1.borrow().pixels()
            )
        };

        images.for_each(|(index, image)|
        {
            let error = Self::pixels_error_early_exit(
                main_pixels.clone(),
                image.borrow().pixels(),
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

    fn subimage(&self, position: Pos2d) -> SubImage<&RgbImage>
    {
        self.image.view(position.x, position.y, self.pixel_size, self.pixel_size)
    }

    fn pixels_error<'a, A, B>(a: A, b: B) -> f64
    where
        A: Iterator<Item=&'a Rgb<u8>>,
        B: Iterator<Item=&'a Rgb<u8>>
    {
        a.zip(b).map(|(a, b)|
        {
            let distance: f64 = a.0.iter().zip(b.0.iter()).map(|(&a, &b)|
            {
                (a as f64 - b as f64).powi(2)
            }).sum();

            distance.sqrt()
        }).sum()
    }

    fn pixels_error_early_exit<'a, A, B>(a: A, b: B, min_bound: f64) -> Option<f64>
    where
        A: Iterator<Item=&'a Rgb<u8>>,
        B: Iterator<Item=&'a Rgb<u8>>
    {
        let error = a.zip(b).map(|(a, b)|
        {
            let distance: f64 = a.0.iter().zip(b.0.iter()).map(|(&a, &b)|
            {
                (a as f64 - b as f64).powi(2)
            }).sum();

            distance.sqrt()
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
