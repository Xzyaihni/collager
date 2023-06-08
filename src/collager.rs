use image::{
    Rgb,
    RgbImage,
    SubImage,
    ImageBuffer,
    GenericImageView,
    imageops::{self, FilterType}
};


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
    pixel_size: u32
}

impl Collager
{
    pub fn new(image: RgbImage, width: u32, pixel_size: u32) -> Self
    {
        let total_width = width * pixel_size;
        let width_scale = total_width as f64 / image.width() as f64;

        let total_height = (image.height() as f64 * width_scale).ceil() as u32;

        let filter_type = FilterType::CatmullRom;

        let image = imageops::resize(&image, total_width, total_height, filter_type);

        let height = total_height / pixel_size;

        Self{image, width, height, pixel_size}
    }

    pub fn collage(&self, images: &[RgbImage]) -> RgbImage
    {
        let indices = self.best_indices(&images);

        self.construct_from_indices(indices, images)
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
        indices: impl Iterator<Item=usize>,
        images: &[RgbImage]
    ) -> RgbImage
    {
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
                let pixel = images[index].get_pixel(x, y);

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

    fn best_indices<'a>(&'a self, images: &'a [RgbImage]) -> impl Iterator<Item=usize> + 'a
    {
        self.positions_iter().map(move |position|
        {
            self.best_fit_index(&images, position)
        })
    }

    fn best_fit_index(&self, images: &[RgbImage], position: Pos2d) -> usize
    {
        let subimage = self.subimage(position).to_image();
        let main_pixels = subimage.pixels();

        let (index, _error) = images.iter().enumerate().map(|(index, image)|
        {
            let this_pixels = image.pixels();

            (index, Self::pixels_error(main_pixels.clone(), this_pixels))
        }).min_by(|(_, a), (_, b)|
        {
            a.partial_cmp(b).unwrap()
        }).expect("images must not be empty");

        index
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
}