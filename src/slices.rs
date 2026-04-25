use rs_math3d::{Vec3d};

pub struct Slice {
    start: Vec3d,
    end: Vec3d,
}

pub struct AnnotatedSlice {
    slice: Slice,
    line_number: u32,
}

pub struct SliceLine {
    line_number: u32,
    slices: Vec<AnnotatedSlice>,
}

impl SliceLine {
    pub fn new(line_number: u32, slices: Vec<AnnotatedSlice>) -> Self {
        Self {
            line_number,
            slices,
        }
    }

    pub fn add(&mut self, slice: AnnotatedSlice) {
        assert!(slice.line_number == self.line_number, "Slice line number does not match SliceLine's line number");
        self.slices.push(slice);
    }
}

pub struct SliceMatrix {
    lines: Vec<SliceLine>,
}

impl SliceMatrix {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn add(&mut self, line: SliceLine) {
        self.lines.push(line);
    }
}

pub struct BasicParams {
    do_grayscale: bool,
    gradient_threshold: u8,
}

pub struct Rectangle {
    top_left: Vec3d,
    bottom_right: Vec3d,
}


fn compute_smoothed_gradient(gray_image: &image::GrayImage, x: u32, y: u32) -> u16 {
    let compute_gradient = |x: u32, y: u32| -> u16 {
        let tl = gray_image.get_pixel(x - 1, y - 1)[0] as f32;
        let tc = gray_image.get_pixel(x, y - 1)[0] as f32;
        let tr = gray_image.get_pixel(x + 1, y - 1)[0] as f32;
        let cl = gray_image.get_pixel(x - 1, y)[0] as f32;
        let cr = gray_image.get_pixel(x + 1, y)[0] as f32;
        let bl = gray_image.get_pixel(x - 1, y + 1)[0] as f32;
        let bc = gray_image.get_pixel(x, y + 1)[0] as f32;
        let br = gray_image.get_pixel(x + 1, y + 1)[0] as f32;

        let sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        let grad_tl_br = br - tl;
        let grad_cl_cr = cr - cl;
        let grad_bl_tr = tr - bl;
        let grad_bc_tc = tc - bc;

        let grad_x = grad_cl_cr + grad_tl_br * sqrt2 + grad_bl_tr * sqrt2;
        let grad_y = -grad_bc_tc + grad_tl_br * sqrt2 - grad_bl_tr * sqrt2;
        let grad_total = grad_x.hypot(grad_y);

        grad_total as u16
    };

    let gradients = [
        compute_gradient(x - 1, y - 1),
        compute_gradient(x, y - 1),
        compute_gradient(x + 1, y - 1),
        compute_gradient(x - 1, y),
        compute_gradient(x, y),
        compute_gradient(x + 1, y),
        compute_gradient(x - 1, y + 1),
        compute_gradient(x, y + 1),
        compute_gradient(x + 1, y + 1),
    ];

    let sum: u32 = gradients.iter().map(|&g| g as u32).sum();
    (sum / 9) as u16
}



fn compute_smoothed_gradient_channel(image: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, x: u32, y: u32, channel: usize) -> u16 {
    let compute_gradient = |x: u32, y: u32| -> u16 {
        let tl = image.get_pixel(x - 1, y - 1)[channel] as f32;
        let tc = image.get_pixel(x, y - 1)[channel] as f32;
        let tr = image.get_pixel(x + 1, y - 1)[channel] as f32;
        let cl = image.get_pixel(x - 1, y)[channel] as f32;
        let cr = image.get_pixel(x + 1, y)[channel] as f32;
        let bl = image.get_pixel(x - 1, y + 1)[channel] as f32;
        let bc = image.get_pixel(x, y + 1)[channel] as f32;
        let br = image.get_pixel(x + 1, y + 1)[channel] as f32;

        let sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        let grad_tl_br = br - tl;
        let grad_cl_cr = cr - cl;
        let grad_bl_tr = tr - bl;
        let grad_bc_tc = tc - bc;

        let grad_x = grad_cl_cr + grad_tl_br * sqrt2 + grad_bl_tr * sqrt2;
        let grad_y = -grad_bc_tc + grad_tl_br * sqrt2 - grad_bl_tr * sqrt2;
        let grad_total = grad_x.hypot(grad_y);

        grad_total as u16
    };

    let gradients = [
        compute_gradient(x - 1, y - 1),
        compute_gradient(x, y - 1),
        compute_gradient(x + 1, y - 1),
        compute_gradient(x - 1, y),
        compute_gradient(x, y),
        compute_gradient(x + 1, y),
        compute_gradient(x - 1, y + 1),
        compute_gradient(x, y + 1),
        compute_gradient(x + 1, y + 1),
    ];

    let sum: u32 = gradients.iter().map(|&g| g as u32).sum();
    (sum / 9) as u16
}


fn emplace_current_slice(current_slice: &mut Option<AnnotatedSlice>, current_line: &mut SliceLine) {
    if let Some(slice) = current_slice.take() {
        current_line.add(slice);
    }
}

pub fn calculate_slices(
    image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    rectangle: Rectangle,
    params: BasicParams,
) -> SliceMatrix {
    // checkt the rectangle is within the bounds of the image
    let (img_width, img_height) = image.dimensions();
    if rectangle.top_left.x < 0.0
        || rectangle.top_left.y < 0.0
        || rectangle.bottom_right.x > img_width as f64
        || rectangle.bottom_right.y > img_height as f64
    {
        panic!("Rectangle is out of bounds of the image");
    }

    let mut slice_matrix = SliceMatrix::new();

    if params.do_grayscale {
        // Convert image to grayscale
        let gray_image = image::imageops::grayscale(&image);
        let mut current_slice = None;
        for y in rectangle.top_left.y as u32 + 2..rectangle.bottom_right.y as u32 - 2 {
            let mut current_line = SliceLine::new(y, Vec::new());
            for x in rectangle.top_left.x as u32 + 2..rectangle.bottom_right.x as u32 - 2 {
                let gradient = compute_smoothed_gradient(&gray_image, x, y);

                if gradient <= params.gradient_threshold as u16 {
                    if current_slice.is_none() {
                        current_slice = Some(AnnotatedSlice {
                            slice: Slice {
                                start: Vec3d::new(x as f64, y as f64, 0.0),
                                end: Vec3d::new(x as f64, y as f64, 0.0),
                            },
                            line_number: y,
                        });
                    } else {
                        if let Some(slice) = &mut current_slice {
                            slice.slice.end.x = x as f64;
                        }
                    }
                }
                else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        return slice_matrix;
    } else {
        let mut current_slice = None;
        for y in rectangle.top_left.y as u32 + 2..rectangle.bottom_right.y as u32 - 2 {
            let mut current_line = SliceLine::new(y, Vec::new());
            for x in rectangle.top_left.x as u32 + 2..rectangle.bottom_right.x as u32 - 2 {
                let gradient_0 = compute_smoothed_gradient_channel(&image, x, y, 0);
                let gradient_1 = compute_smoothed_gradient_channel(&image, x, y, 1);
                let gradient_2 = compute_smoothed_gradient_channel(&image, x, y, 2);

                if gradient_0 <= params.gradient_threshold as u16
                    && gradient_1 <= params.gradient_threshold as u16
                    && gradient_2 <= params.gradient_threshold as u16{
                    if current_slice.is_none() {
                        current_slice = Some(AnnotatedSlice {
                            slice: Slice {
                                start: Vec3d::new(x as f64, y as f64, 0.0),
                                end: Vec3d::new(x as f64, y as f64, 0.0),
                            },
                            line_number: y,
                        });
                    } else {
                        if let Some(slice) = &mut current_slice {
                            slice.slice.end.x = x as f64;
                        }
                    }
                }
                else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        return slice_matrix;
    }
}
