#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::math::CoordinatedCircle;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use crate::math::Point;
use crate::math::Rectangle as MathRectangle;
use num_traits::ToPrimitive;
use rs_math3d::{FloatVector, Vec3d};

use super::{
    AnnotatedSlice, CachedData, Rectangle, RgbPixelReader, Slice, SliceLine, WrappedRgbImage,
    f64_from_usize, u32_from_i32, usize_from_f64, usize_from_u32,
};

#[derive(Clone)]
pub struct SliceMatrix {
    lines: Vec<SliceLine>,
    image: WrappedRgbImage,
}

impl SliceMatrix {
    pub(super) fn lines(&self) -> &Vec<SliceLine> {
        &self.lines
    }

    pub(super) fn image(&self) -> &WrappedRgbImage {
        &self.image
    }
}

impl SliceMatrix {
    #[must_use]
    pub fn new(image: WrappedRgbImage) -> Self {
        Self {
            lines: Vec::new(),
            image,
        }
    }

    pub fn add(&mut self, line: SliceLine) {
        self.lines.push(line);
    }

    #[must_use]
    pub fn deduce_longest_distance_point(
        &self,
        point: &CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        let mut longest_distance = 0.0;
        let mut longest_distance_point = None;
        for line in &self.lines {
            for slice in line.slices() {
                let inner_slice = slice.get_slice();
                let top_left_corner = inner_slice.get_start();
                let mut bottom_left_corner = inner_slice.get_start().clone();
                bottom_left_corner.set_y(bottom_left_corner.get_y() + 1.0);
                let top_right_corner = inner_slice.get_end();
                let mut bottom_right_corner = inner_slice.get_end().clone();
                bottom_right_corner.set_y(bottom_right_corner.get_y() + 1.0);

                let top_left_distance = point.distance_to(top_left_corner);
                let bottom_left_distance = point.distance_to(&bottom_left_corner);
                let top_right_distance = point.distance_to(top_right_corner);
                let bottom_right_distance = point.distance_to(&bottom_right_corner);

                if top_left_distance > longest_distance {
                    longest_distance = top_left_distance;
                    longest_distance_point = Some(top_left_corner.clone());
                }
                if bottom_left_distance > longest_distance {
                    longest_distance = bottom_left_distance;
                    longest_distance_point = Some(bottom_left_corner.clone());
                }
                if top_right_distance > longest_distance {
                    longest_distance = top_right_distance;
                    longest_distance_point = Some(top_right_corner.clone());
                }
                if bottom_right_distance > longest_distance {
                    longest_distance = bottom_right_distance;
                    longest_distance_point = Some(bottom_right_corner.clone());
                }
            }
        }
        longest_distance_point
    }

    #[must_use]
    pub fn get_top_line_number(&self) -> Option<usize> {
        self.lines
            .first()
            .map(super::slice_line::SliceLine::line_number)
    }

    #[must_use]
    pub fn get_bottom_line_number(&self) -> Option<usize> {
        self.lines
            .last()
            .map(super::slice_line::SliceLine::line_number)
    }

    #[must_use]
    pub fn get_top_left_slice(&self) -> Option<SliceLine> {
        let annotated_slice = self
            .lines
            .first()
            .and_then(|line| line.slices().first().cloned());
        annotated_slice.map(|slice| SliceLine::new(slice.line_number(), vec![slice]))
    }

    #[must_use]
    pub fn get_line_below(&self, line_number: usize) -> Option<&SliceLine> {
        self.lines
            .iter()
            .find(|line| line.line_number() == line_number + 1)
    }

    #[must_use]
    pub fn get_line_above(&self, line_number: usize) -> Option<&SliceLine> {
        let target = line_number.checked_sub(1)?;
        self.lines.iter().find(|line| line.line_number() == target)
    }

    #[must_use]
    pub fn get_line(&self, line_number: usize) -> Option<&SliceLine> {
        self.lines
            .iter()
            .find(|line| line.line_number() == line_number)
    }

    #[must_use]
    pub fn find_touching_slices(&self, line: &SliceLine, direction: i32) -> Option<SliceLine> {
        let touching_line = if direction == -1 {
            self.get_line_below(line.line_number())
        } else {
            self.get_line_above(line.line_number())
        };
        touching_line?;
        let mut result_line = SliceLine::new(touching_line.unwrap().line_number(), Vec::new());
        for slice in line.slices() {
            for touching_slice in touching_line.unwrap().slices() {
                let does_touch = slice.touches(touching_slice);
                if does_touch {
                    result_line.add(touching_slice.clone());
                }
            }
        }
        if result_line.slices().is_empty() {
            None
        } else {
            Some(result_line)
        }
    }

    fn insert_where_needed(&mut self, line: SliceLine) {
        if let Some(pos) = self
            .lines
            .iter()
            .position(|l| l.line_number() > line.line_number())
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
            .find(|l| l.line_number() == line.line_number())
        {
            existing_line.add_slices(&line);
        } else {
            self.insert_where_needed(line);
        }
    }

    pub fn remove_slices(&mut self, line: &SliceLine) {
        if let Some(existing_line) = self
            .lines
            .iter_mut()
            .find(|l| l.line_number() == line.line_number())
        {
            existing_line
                .slices_mut()
                .retain(|slice| !line.slices().contains(slice));
            if existing_line.slices().is_empty() {
                self.lines.retain(|l| l.line_number() != line.line_number());
            }
        }
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> Rectangle {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for line in &self.lines {
            for slice in line.slices() {
                min_x = min_x.min(slice.get_slice().get_start().get_x());
                max_x = max_x.max(slice.get_slice().get_end().get_x());
                min_y = min_y.min(slice.get_slice().get_start().get_y());
                max_y = max_y.max(slice.get_slice().get_end().get_y());
            }
        }

        Rectangle::new(Vec3d::new(min_x, min_y, 0.0), Vec3d::new(max_x, max_y, 0.0))
    }

    pub fn calculate_cached_data(&self) -> CachedData {
        let mut masses = Vec::new();
        let mut tl = Vec3d::new(f64::INFINITY, f64::INFINITY, 0.0);
        let mut br = Vec3d::new(f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0);
        let mut colors = Vec::new();
        let rgb_reader =
            RgbPixelReader::from_guard(self.image.get_mat()).expect("failed to access RGB image");
        for line in &self.lines {
            for slice in line.slices() {
                masses.push((slice.get_mass(), slice.get_midpoint()));
                let inner_slice = slice.get_slice();
                let left_point = inner_slice.get_start();
                let right_point = inner_slice.get_end();
                tl.x = tl.x.min(left_point.get_x());
                tl.y = tl.y.min(left_point.get_y());
                br.x = br.x.max(right_point.get_x());
                br.y = br.y.max(right_point.get_y());
                for x in u32_from_i32(
                    left_point
                        .get_x()
                        .to_i32()
                        .expect("x coordinate should fit i32"),
                )
                    ..=u32_from_i32(
                        right_point
                            .get_x()
                            .to_i32()
                            .expect("x coordinate should fit i32"),
                    )
                {
                    colors.push(unsafe {
                        rgb_reader
                            .pixel_unchecked(usize_from_u32(x), usize_from_f64(left_point.get_y()))
                    });
                }
            }
        }
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let tl_coordinated = CoordinatedPoint::new(global_coordinate_system.clone(), tl);
        let br_coordinated = CoordinatedPoint::new(global_coordinate_system.clone(), br);
        let bounding_box =
            CoordinatedRectangle::new(tl_coordinated.clone(), br_coordinated.clone());
        let center_of_mass =
            masses
                .iter()
                .fold(Vec3d::new(0.0, 0.0, 0.0), |acc, (mass, midpoint)| {
                    let deref_mass = *mass;
                    acc + *midpoint * deref_mass
                })
                / masses.iter().map(|(mass, _)| *mass).sum::<f64>();
        let coordinated_center_of_mass =
            CoordinatedPoint::new(global_coordinate_system.clone(), center_of_mass);
        let max_radius_from_center = masses
            .iter()
            .map(|(_, midpoint)| (*midpoint - center_of_mass).length())
            .fold(0.0, f64::max);
        let bounding_circle =
            CoordinatedCircle::new(coordinated_center_of_mass.clone(), max_radius_from_center);
        let area = masses.iter().map(|(mass, _)| *mass).sum::<f64>();

        let average_color_vec = colors.iter().fold(Vec3d::new(0.0, 0.0, 0.0), |acc, pixel| {
            acc + Vec3d::new(
                f64::from(pixel[0]),
                f64::from(pixel[1]),
                f64::from(pixel[2]),
            )
        }) / f64_from_usize(colors.len());
        CachedData::new(
            bounding_box,
            bounding_circle,
            coordinated_center_of_mass,
            area,
            average_color_vec,
        )
    }

    #[must_use]
    pub fn contains_point(&self, point: &CoordinatedPoint) -> bool {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let global_point = point.convert_to(global_coordinate_system.clone());
        let y_coordinate = global_point.get_y().floor();
        let first_line_number = f64_from_usize(self.lines.first().unwrap().line_number());
        if y_coordinate < first_line_number {
            return false;
        }
        let last_line_number = f64_from_usize(self.lines.last().unwrap().line_number());
        if y_coordinate > last_line_number {
            return false;
        }
        let line = &self.lines[usize_from_f64(y_coordinate - first_line_number)];
        let point_rectangle_tl = Point::new(Vec3d::new(
            global_point.get_x().floor(),
            global_point.get_y().floor(),
            0.0,
        ));
        let point_rectangle_br = Point::new(Vec3d::new(
            (global_point.get_x() + 1.0).floor(),
            (global_point.get_y() + 1.0).floor(),
            0.0,
        ));
        let point_rectangle =
            MathRectangle::new(point_rectangle_tl.point, point_rectangle_br.point);
        for slice in line.get_slices() {
            if point_rectangle.overlaps(&slice.slice().get_global_rectangle().get_local_rectangle())
            {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn get_slice_lines(&self) -> Vec<SliceLine> {
        self.lines.clone()
    }

    pub fn expand(&mut self) {
        if self.lines.is_empty() {
            return;
        }

        for line in &mut self.lines {
            line.expand_by(1.0);
            line.merge_overlapping_slices();
        }

        let original_lines = self.lines.clone();
        let original_len = original_lines.len();
        let mut borrowed_neighbors = vec![Vec::<Vec<AnnotatedSlice>>::new(); original_len];

        for index in 0..original_len {
            let line_number = original_lines[index].line_number();
            if index > 0 {
                let mut upper = original_lines[index - 1].slices().clone();
                for slice in &mut upper {
                    let delta = f64_from_usize(line_number) - f64_from_usize(slice.line_number());
                    slice.set_line_number(line_number);
                    let new_start = slice.slice().get_start().plus(Vec3d::new(0.0, delta, 0.0));
                    let new_end = slice.slice().get_end().plus(Vec3d::new(0.0, delta, 0.0));
                    slice.set_slice(Slice::new(&new_start, &new_end));
                }
                borrowed_neighbors[index].push(upper);
            }
            if index + 1 < original_len {
                let mut lower = original_lines[index + 1].slices().clone();
                for slice in &mut lower {
                    let delta = f64_from_usize(line_number) - f64_from_usize(slice.line_number());
                    slice.set_line_number(line_number);
                    let new_start = slice.slice().get_start().plus(Vec3d::new(0.0, delta, 0.0));
                    let new_end = slice.slice().get_end().plus(Vec3d::new(0.0, delta, 0.0));
                    slice.set_slice(Slice::new(&new_start, &new_end));
                }
                borrowed_neighbors[index].push(lower);
            }
        }

        let mut inserted_top = false;
        if let Some(first_line) = original_lines.first()
            && first_line.line_number() > 0
        {
            let mut new_top_slices = first_line.slices().clone();
            for slice in &mut new_top_slices {
                slice.set_line_number(slice.line_number() - 1);
                let new_start = slice.slice().get_start().plus(Vec3d::new(0.0, -1.0, 0.0));
                let new_end = slice.slice().get_end().plus(Vec3d::new(0.0, -1.0, 0.0));
                slice.set_slice(Slice::new(&new_start, &new_end));
            }
            let mut new_top_line = SliceLine::new(first_line.line_number() - 1, new_top_slices);
            new_top_line.merge_overlapping_slices();
            self.insert_where_needed(new_top_line);
            inserted_top = true;
        }

        if let Some(last_line) = original_lines.last() {
            let mut new_bottom_slices = last_line.slices().clone();
            for slice in &mut new_bottom_slices {
                slice.set_line_number(slice.line_number() + 1);
                let new_start = slice.slice().get_start().plus(Vec3d::new(0.0, 1.0, 0.0));
                let new_end = slice.slice().get_end().plus(Vec3d::new(0.0, 1.0, 0.0));
                slice.set_slice(Slice::new(&new_start, &new_end));
            }
            let mut new_bottom_line =
                SliceLine::new(last_line.line_number() + 1, new_bottom_slices);
            new_bottom_line.merge_overlapping_slices();
            self.insert_where_needed(new_bottom_line);
        }

        for (index, neighbor_groups) in borrowed_neighbors.into_iter().enumerate() {
            let target_index = if inserted_top { index + 1 } else { index };
            for neighbor_group in neighbor_groups {
                self.lines[target_index].slices_mut().extend(neighbor_group);
            }
            self.lines[target_index].maintain_slices();
            self.lines[target_index].merge_overlapping_slices();
        }
    }
}
