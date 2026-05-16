use core::f64;

use rs_math3d::FloatVector;
use rs_math3d::Vec3d;

use crate::math::CoordinatedPoint;
use crate::math::Rectangle as OtherRectangle;
use crate::math::WrappedCoordinateSystem;

#[derive(Clone)]
pub struct Slice {
    start: CoordinatedPoint,
    end: CoordinatedPoint,
}

impl PartialEq for Slice {
    fn eq(&self, other: &Self) -> bool {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let self_start_global = self.start.convert_to(global_coordinate_system.clone());
        let self_end_global = self.end.convert_to(global_coordinate_system.clone());
        let other_start_global = other.start.convert_to(global_coordinate_system.clone());
        let other_end_global = other.end.convert_to(global_coordinate_system.clone());
        let start_diff = (self_start_global.get_local_point() - other_start_global.get_local_point()).length();
        let end_diff = (self_end_global.get_local_point() - other_end_global.get_local_point()).length();
        start_diff < 1e-6 && end_diff < 1e-6
    }
}

impl Slice {
    pub fn new(start: CoordinatedPoint, end: CoordinatedPoint) -> Self {
        Self { start, end }
    }

    pub fn get_start(&self) -> CoordinatedPoint {
        self.start.clone()
    }

    pub fn get_end(&self) -> CoordinatedPoint {
        self.end.clone()
    }

    pub fn convert_to_global(&self) -> Slice {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        Slice {
            start: self.start.convert_to(global_coordinate_system.clone()),
            end: self.end.convert_to(global_coordinate_system.clone()),
        }
    }

    pub fn convert_to(&self, coordinate_system: WrappedCoordinateSystem) -> Slice {
        Slice {
            start: self.start.convert_to(coordinate_system.clone()),
            end: self.end.convert_to(coordinate_system.clone()),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct AnnotatedSlice {
    slice: Slice,
    line_number: u32,
}

impl AnnotatedSlice {
    pub fn touches(&self, other: &AnnotatedSlice) -> bool {
        // Check if the slices are on adjacent lines
        if (self.line_number as i32 - other.line_number as i32).abs() != 1 {
            return false;
        }
        // Check if the slices overlap in the x-axis
        let self_start_x = self.slice.get_start().get_x();
        let self_end_x = self.slice.get_end().get_x();
        let other_start_x = other.slice.get_start().get_x();
        let other_end_x = other.slice.get_end().get_x();

        !(self_end_x < other_start_x || self_start_x > other_end_x)
    }

    pub fn get_mass(&self) -> f64 {
        self.slice.get_end().get_x() - self.slice.get_start().get_x() + 1.0
    }

    pub fn get_midpoint(&self) -> Vec3d {
        Vec3d::new(
            (self.slice.get_start().get_x() + self.slice.get_end().get_x()) / 2.0,
            self.slice.get_start().get_y(),
            0.0,
        )
    }

    pub fn get_slice(&self) -> Slice {
        self.slice.clone()
    }
}

#[derive(Clone)]
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

    pub fn maintain_slices(&mut self) {
        // remove duplicates of the same slice
        self.slices.sort_by(|a, b| {
            let a_start_x = a.slice.get_start().get_x();
            let a_end_x = a.slice.get_end().get_x();
            let b_start_x = b.slice.get_start().get_x();
            let b_end_x = b.slice.get_end().get_x();
            a_start_x
                .partial_cmp(&b_start_x)
                .unwrap()
                .then(a_end_x.partial_cmp(&b_end_x).unwrap())
        });
        self.slices.dedup();
    }

    pub fn add(&mut self, slice: AnnotatedSlice) {
        assert!(
            slice.line_number == self.line_number,
            "Slice line number does not match SliceLine's line number"
        );
        self.slices.push(slice);
        self.maintain_slices();
    }

    pub fn add_slices(&mut self, slice_line: &SliceLine) {
        assert!(
            slice_line.line_number == self.line_number,
            "Slice line number does not match SliceLine's line number"
        );
        self.slices.extend(slice_line.slices.clone());
        self.maintain_slices();
    }

    pub fn get_line_number(&self) -> u32 {
        self.line_number
    }
}

#[derive(Clone)]
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

    pub fn get_top_line_number(&self) -> Option<u32> {
        self.lines.first().map(|line| line.line_number)
    }

    pub fn get_bottom_line_number(&self) -> Option<u32> {
        self.lines.last().map(|line| line.line_number)
    }

    pub fn get_top_left_slice(&self) -> Option<SliceLine> {
        let annotated_slice = self
            .lines
            .first()
            .and_then(|line| line.slices.first().cloned());
        annotated_slice.map(|slice| SliceLine::new(slice.line_number, vec![slice]))
    }

    pub fn get_line_below(&self, line_number: u32) -> Option<&SliceLine> {
        self.lines
            .iter()
            .find(|line| line.line_number == line_number + 1)
    }

    pub fn get_line_above(&self, line_number: u32) -> Option<&SliceLine> {
        self.lines
            .iter()
            .find(|line| line.line_number == line_number - 1)
    }

    pub fn get_line(&self, line_number: u32) -> Option<&SliceLine> {
        self.lines
            .iter()
            .find(|line| line.line_number == line_number)
    }

    pub fn find_touching_slices(&self, line: &SliceLine, direction: i32) -> Option<SliceLine> {
        let touching_line = if direction == -1 {
            self.get_line_below(line.line_number)
        } else {
            self.get_line_above(line.line_number)
        };
        touching_line?;
        let mut result_line = SliceLine::new(touching_line.unwrap().line_number, Vec::new());
        for slice in &line.slices {
            for touching_slice in &touching_line.unwrap().slices {
                let does_touch = slice.touches(touching_slice);
                if does_touch {
                    result_line.add(touching_slice.clone());
                }
            }
        }
        if result_line.slices.is_empty() {
            None
        } else {
            Some(result_line)
        }
    }

    fn insert_where_needed(&mut self, line: SliceLine) {
        if let Some(pos) = self
            .lines
            .iter()
            .position(|l| l.line_number > line.line_number)
        {
            self.lines.insert(pos, line);
        } else {
            self.lines.push(line);
        }
    }

    pub fn add_slices(&mut self, line: SliceLine) {
        if let Some(existing_line) = self
            .lines
            .iter_mut()
            .find(|l| l.line_number == line.line_number)
        {
            existing_line.add_slices(&line);
        } else {
            self.insert_where_needed(line);
        }
    }

    pub fn remove_slices(&mut self, line: SliceLine) {
        if let Some(existing_line) = self
            .lines
            .iter_mut()
            .find(|l| l.line_number == line.line_number)
        {
            existing_line
                .slices
                .retain(|slice| !line.slices.contains(slice));
            if existing_line.slices.is_empty() {
                self.lines.retain(|l| l.line_number != line.line_number);
            }
        }
    }

    pub fn get_bounding_box(&self) -> Rectangle {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for line in &self.lines {
            for slice in &line.slices {
                min_x = min_x.min(slice.get_slice().get_start().get_x());
                max_x = max_x.max(slice.get_slice().get_end().get_x());
                min_y = min_y.min(slice.get_slice().get_start().get_y());
                max_y = max_y.max(slice.get_slice().get_end().get_y());
            }
        }

        Rectangle {
            top_left: Vec3d::new(min_x, min_y, 0.0),
            bottom_right: Vec3d::new(max_x, max_y, 0.0),
        }
    }

    pub fn calculate_cached_data(&self) -> CachedData {
        let mut masses = Vec::new();
        let mut tl = Vec3d::new(f64::INFINITY, f64::INFINITY, 0.0);
        let mut br = Vec3d::new(f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0);
        for line in &self.lines {
            for slice in &line.slices {
                masses.push((slice.get_mass(), slice.get_midpoint()));
                let left_point = slice.get_slice().get_start();
                let right_point = slice.get_slice().get_end();
                tl.x = tl.x.min(left_point.get_x());
                tl.y = tl.y.min(left_point.get_y());
                br.x = br.x.max(right_point.get_x());
                br.y = br.y.max(right_point.get_y());
            }
        }
        let bounding_box = OtherRectangle::new(tl, br);
        let center_of_mass =
            masses
                .iter()
                .fold(Vec3d::new(0.0, 0.0, 0.0), |acc, (mass, midpoint)| {
                    let deref_mass = *mass;
                    acc + *midpoint * deref_mass
                })
                / masses.iter().map(|(mass, _)| *mass).sum::<f64>();
        let max_radius_from_center = masses
            .iter()
            .map(|(_, midpoint)| (*midpoint - center_of_mass).length())
            .fold(0.0, f64::max);
        let bounding_circle = crate::math::Circle::new(center_of_mass, max_radius_from_center);
        CachedData::new(bounding_box, bounding_circle, center_of_mass)
    }

    pub fn contains_point(&self, point: CoordinatedPoint) -> bool {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let global_point = point.convert_to(global_coordinate_system.clone());
        for line in &self.lines {
            for slice in &line.slices {
                let slice_start = slice.get_slice().get_start();
                let slice_end = slice.get_slice().get_end();
                let global_start = slice_start.convert_to(global_coordinate_system.clone());
                let global_end = slice_end.convert_to(global_coordinate_system.clone());
                if global_point.get_y() == global_start.get_y()
                    && global_point.get_x() >= global_start.get_x()
                    && global_point.get_x() <= global_end.get_x()
                {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for SliceMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct CachedData {
    bounding_box: OtherRectangle,
    bounding_circle: crate::math::Circle,
    center_of_mass: Vec3d,
}

impl CachedData {
    pub fn new(
        bounding_box: OtherRectangle,
        bounding_circle: crate::math::Circle,
        center_of_mass: Vec3d,
    ) -> Self {
        Self {
            bounding_box,
            bounding_circle,
            center_of_mass,
        }
    }

    pub fn get_bounding_box(&self) -> OtherRectangle {
        self.bounding_box.clone()
    }

    pub fn get_bounding_circle(&self) -> crate::math::Circle {
        self.bounding_circle.clone()
    }

    pub fn get_center_of_mass(&self) -> Vec3d {
        self.center_of_mass
    }
}

pub struct BasicParams {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl BasicParams {
    pub fn new(do_grayscale: bool, gradient_threshold: u8) -> Self {
        Self {
            do_grayscale,
            gradient_threshold,
        }
    }
}

pub struct Rectangle {
    top_left: Vec3d,
    bottom_right: Vec3d,
}

impl Rectangle {
    pub fn new(top_left: Vec3d, bottom_right: Vec3d) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    pub fn get_top_left(&self) -> Vec3d {
        self.top_left
    }

    pub fn get_bottom_right(&self) -> Vec3d {
        self.bottom_right
    }

    pub fn get_area(&self) -> f64 {
        let width = self.bottom_right.x - self.top_left.x + 1.0;
        let height = self.bottom_right.y - self.top_left.y + 1.0;
        width * height
    }
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

fn compute_smoothed_gradient_channel(
    image: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    x: u32,
    y: u32,
    channel: usize,
) -> u16 {
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
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        for y in rectangle.top_left.y as u32 + 2..rectangle.bottom_right.y as u32 - 2 {
            let mut current_line = SliceLine::new(y, Vec::new());
            for x in rectangle.top_left.x as u32 + 2..rectangle.bottom_right.x as u32 - 2 {
                let gradient = compute_smoothed_gradient(&gray_image, x, y);


                if gradient <= params.gradient_threshold as u16 {
                    if current_slice.is_none() {
                        current_slice = Some(AnnotatedSlice {
                            slice: Slice {
                                start: CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0)),
                                end: CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0)),
                            },
                            line_number: y,
                        });
                    } else {
                        if let Some(slice) = &mut current_slice {
                            slice.slice.end = CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0));
                        }
                    }
                } else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        slice_matrix
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
                    && gradient_2 <= params.gradient_threshold as u16
                {
                    let global_coordinate_system = WrappedCoordinateSystem::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(1.0, 0.0, 0.0),
                        Vec3d::new(0.0, 1.0, 0.0),
                    );
                    if current_slice.is_none() {
                        current_slice = Some(AnnotatedSlice {
                            slice: Slice {
                                start: CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0)),
                                end: CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0)),
                            },
                            line_number: y,
                        });
                    } else {
                        if let Some(slice) = &mut current_slice {
                            slice.slice.end = CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(x as f64, y as f64, 0.0));
                        }
                    }
                } else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        slice_matrix
    }
    // print all lines of the slice matrix.
}

fn go_direction(
    slice_matrix: &mut SliceMatrix,
    connected_matrix: &mut SliceMatrix,
    direction: i32,
) -> bool {
    if direction == -1 {
        let mut top_line_number = connected_matrix.get_top_line_number();
        if top_line_number.is_none() {
            let tl_slice = slice_matrix.get_top_left_slice();
            if let Some(tl_slice) = tl_slice {
                connected_matrix.add_slices(tl_slice.clone());
                slice_matrix.remove_slices(tl_slice.clone());
            } else {
                return false;
            }
            top_line_number = connected_matrix.get_top_line_number();
        }
        let mut added_something = false;
        let mut current_line = top_line_number.expect("Did not find a top line number");
        loop {
            let next_line = connected_matrix.get_line(current_line);
            if let Some(line) = next_line {
                let touching_slices = slice_matrix.find_touching_slices(line, -1);
                if let Some(touching_slices) = touching_slices {
                    connected_matrix.add_slices(touching_slices.clone());
                    slice_matrix.remove_slices(touching_slices.clone());
                    added_something = true;
                    current_line = touching_slices.get_line_number();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return added_something;
    } else if direction == 1 {
        let mut bottom_line_number = connected_matrix.get_bottom_line_number();
        if bottom_line_number.is_none() {
            let tl_slice = slice_matrix.get_top_left_slice();
            if let Some(tl_slice) = tl_slice {
                connected_matrix.add_slices(tl_slice.clone());
                slice_matrix.remove_slices(tl_slice.clone());
            } else {
                return false;
            }
            bottom_line_number = connected_matrix.get_bottom_line_number();
        }
        let mut added_something = false;
        let mut current_line = bottom_line_number.expect("Did not find a bottom line number");
        loop {
            let next_line = connected_matrix.get_line(current_line);
            if let Some(line) = next_line {
                let touching_slices = slice_matrix.find_touching_slices(line, 1);
                if let Some(touching_slices) = touching_slices {
                    connected_matrix.add_slices(touching_slices.clone());
                    slice_matrix.remove_slices(touching_slices.clone());
                    added_something = true;
                    current_line = touching_slices.get_line_number();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return added_something;
    }
    false
}

fn find_next_connected_slice_matrix(slice_matrix: &mut SliceMatrix) -> Option<SliceMatrix> {
    // Placeholder implementation - in a real implementation, this would perform a search to find connected slices
    let mut connected_matrix = SliceMatrix::new();
    let mut direction = -1; // -1 = go top to bottom, 1 = go bottom to top
    let mut found_nothing_counter = 0;
    loop {
        let added_something = go_direction(slice_matrix, &mut connected_matrix, direction);
        if direction == -1 {
            direction = 1;
        } else {
            direction = -1;
        }
        if added_something {
            found_nothing_counter = 0;
        } else {
            found_nothing_counter += 1;
        }
        if found_nothing_counter >= 2 {
            break;
        }
    }
    if !slice_matrix.lines.is_empty() {
        Some(connected_matrix)
    } else {
        None
    }
}

pub fn find_connected_slices(slice_matrix: &mut SliceMatrix) -> Vec<SliceMatrix> {
    let mut connected_matrices = Vec::new();

    while let Some(connected_slice_matrix) = find_next_connected_slice_matrix(slice_matrix) {
        connected_matrices.push(connected_slice_matrix);
    }

    connected_matrices
}
