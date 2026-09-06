use crate::{
    math::CoordinatedCircle,
    math::CoordinatedPoint,
    math::CoordinatedRectangle,
    math::Rectangle,
    math::WrappedCoordinateSystem,
    slices::Rectangle as SliceRectangle,
    slices::RelativeRectangle,
    slices::{CachedData, SliceMatrix},
};

use rs_math3d::Vec3d;

use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CachedRelativeData {
    bounding_box: CoordinatedRectangle,
    bounding_circle: CoordinatedCircle,
    center_of_mass: CoordinatedPoint,
    area: f64,
    average_color: Vec3d,
}

impl CachedRelativeData {
    pub fn new(
        bounding_box: CoordinatedRectangle,
        bounding_circle: CoordinatedCircle,
        center_of_mass: CoordinatedPoint,
        area: f64,
        average_color: Vec3d,
    ) -> Self {
        Self {
            bounding_box,
            bounding_circle,
            center_of_mass,
            area,
            average_color,
        }
    }

    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        self.bounding_box.clone()
    }

    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        self.bounding_circle.clone()
    }

    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        self.center_of_mass.clone()
    }

    pub fn get_area(&self) -> f64 {
        self.area
    }

    pub fn get_average_color_vec(&self) -> Vec3d {
        self.average_color
    }
}

#[derive(Clone)]
struct Mosaic {
    slice_matrix: SliceMatrix,
    cached_data: Option<CachedData>,
}

impl Mosaic {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            slice_matrix,
            cached_data: None,
        }
    }

    pub fn get_bounding_box(&mut self) -> CoordinatedRectangle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_box()
    }

    pub fn get_bounding_circle(&mut self) -> CoordinatedCircle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_circle()
    }

    pub fn get_center_of_mass(&mut self) -> CoordinatedPoint {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_center_of_mass()
    }

    pub fn get_area(&mut self) -> f64 {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_area()
    }



    pub fn deduce_longest_distance_point(
        &self,
        point: CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        self.slice_matrix.deduce_longest_distance_point(point)
    }

    fn calculate_cached_data(&mut self) {
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
    }

    fn contains_point(&self, point: CoordinatedPoint) -> bool {
        // convert the point to the global coordinate system and check if it is contained in the mosaic
        self.slice_matrix.contains_point(point)
    }

    fn get_average_color(&mut self) -> Vec3d {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_average_color_vec()
    }
}

#[derive(Clone)]
struct RelativeMosaic {
    mosaic: WrappedMosaic,
    absolute_rectangle: Rectangle,
    cached_relative_data: Option<CachedRelativeData>,
}

impl RelativeMosaic {
    pub fn new(mosaic: WrappedMosaic, absolute_rectangle: Rectangle) -> Self {
        Self {
            mosaic,
            absolute_rectangle,
            cached_relative_data: None,
        }
    }

    pub fn get_bounding_box(&mut self) -> CoordinatedRectangle {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_bounding_box()
    }

    pub fn get_bounding_circle(&mut self) -> CoordinatedCircle {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_bounding_circle()
    }

    pub fn get_center_of_mass(&mut self) -> CoordinatedPoint {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_center_of_mass()
    }

    pub fn get_area(&mut self) -> f64 {
        self.calculate_cached_relative_data();
        self.cached_relative_data.as_ref().unwrap().get_area()
    }

    pub fn get_average_color(&mut self) -> Vec3d {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_average_color_vec()
    }

    pub fn get_absolute_rectangle(&self) -> Rectangle {
        self.absolute_rectangle.clone()
    }

    pub fn get_mosaic(&self) -> WrappedMosaic {
        self.mosaic.clone()
    }

    fn calculate_cached_relative_data(&mut self) {
        if self.cached_relative_data.is_some() {
            return;
        }
        let relative_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let top_left = self.absolute_rectangle.get_top_left();
        let bottom_right = self.absolute_rectangle.get_bottom_right();
        let width = bottom_right.x - top_left.x;
        let height = bottom_right.y - top_left.y;
        assert!(width > 0.0, "Absolute rectangle width must be positive");
        assert!(height > 0.0, "Absolute rectangle height must be positive");
        let absolute_area = width * height;
        let relative_bounding_box =
            self.get_relative_bounding_box(relative_coordinate_system.clone());
        let bounding_circle = self.mosaic.get_bounding_circle();
        let relative_circle_center = Self::map_to_relative_point(
            bounding_circle.get_center(),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        );
        let radius = bounding_circle.get_radius();
        let circle_center_global = bounding_circle
            .get_center()
            .convert_to(relative_coordinate_system.clone())
            .get_local_point();
        let relative_x_radius = Self::map_to_relative_point(
            CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                circle_center_global + Vec3d::new(radius, 0.0, 0.0),
            ),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        )
        .distance_to(relative_circle_center.clone());
        let relative_y_radius = Self::map_to_relative_point(
            CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                circle_center_global + Vec3d::new(0.0, radius, 0.0),
            ),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        )
        .distance_to(relative_circle_center.clone());
        let relative_bounding_circle = CoordinatedCircle::new(
            relative_circle_center,
            relative_x_radius.max(relative_y_radius),
        );
        let relative_center_of_mass = Self::map_to_relative_point(
            self.mosaic.get_center_of_mass(),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        );
        let relative_area = self.mosaic.get_area() / absolute_area;
        self.cached_relative_data = Some(CachedRelativeData::new(
            relative_bounding_box,
            relative_bounding_circle,
            relative_center_of_mass,
            relative_area,
            self.mosaic.get_average_color(),
        ));
    }

    fn map_to_relative_point(
        point: CoordinatedPoint,
        relative_coordinate_system: &WrappedCoordinateSystem,
        top_left: Vec3d,
        width: f64,
        height: f64,
    ) -> CoordinatedPoint {
        let converted_point = point.convert_to(relative_coordinate_system.clone());
        CoordinatedPoint::new(
            relative_coordinate_system.clone(),
            Vec3d::new(
                (converted_point.get_x() - top_left.x) / width,
                (converted_point.get_y() - top_left.y) / height,
                0.0,
            ),
        )
    }

    fn get_relative_bounding_box(
        &self,
        relative_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedRectangle {
        let bounding_box = SliceRectangle::new_from_math_rectangle(
            self.mosaic.get_bounding_box().to_global_rectangle(),
        );
        let absolute_rectangle =
            SliceRectangle::new_from_math_rectangle(self.absolute_rectangle.clone());
        let unit_scale_reference =
            SliceRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 0.0));
        let relative_rectangle =
            RelativeRectangle::new_from_rectangles(bounding_box, absolute_rectangle);
        let relative_bounding_box =
            relative_rectangle.multiply_with_rectangle(unit_scale_reference);
        CoordinatedRectangle::new(
            CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                relative_bounding_box.get_top_left(),
            ),
            CoordinatedPoint::new(
                relative_coordinate_system,
                relative_bounding_box.get_bottom_right(),
            ),
        )
    }
}

#[derive(Clone)]
pub struct WrappedMosaic {
    mosaic: Arc<Mutex<Mosaic>>,
}

impl WrappedMosaic {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            mosaic: Arc::new(Mutex::new(Mosaic::new(slice_matrix))),
        }
    }

    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_box()
    }

    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_circle()
    }

    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_center_of_mass()
    }

    pub fn get_area(&self) -> f64 {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_area()
    }

    pub fn contains_point(&self, point: CoordinatedPoint) -> bool {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.contains_point(point)
    }

    pub fn deduce_longest_distance_point(
        &self,
        point: CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.deduce_longest_distance_point(point)
    }
    pub fn get_slice_matrix(&self) -> SliceMatrix {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.slice_matrix.clone()
    }

    pub fn get_average_color(&self) -> Vec3d {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_average_color()
    }
}

#[derive(Clone)]
pub struct WrappedRelativeMosaic {
    relative_mosaic: Arc<Mutex<RelativeMosaic>>,
}

impl WrappedRelativeMosaic {
    pub fn new(mosaic: WrappedMosaic, absolute_rectangle: Rectangle) -> Self {
        Self {
            relative_mosaic: Arc::new(Mutex::new(RelativeMosaic::new(mosaic, absolute_rectangle))),
        }
    }

    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_bounding_box()
    }

    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_bounding_circle()
    }

    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_center_of_mass()
    }

    pub fn get_area(&self) -> f64 {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_area()
    }

    pub fn get_average_color(&self) -> Vec3d {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_average_color()
    }

    pub fn get_absolute_rectangle(&self) -> Rectangle {
        let relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_absolute_rectangle()
    }

    pub fn get_mosaic(&self) -> WrappedMosaic {
        let relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_mosaic()
    }

    pub fn shares_identity_with(&self, other: &WrappedRelativeMosaic) -> bool {
        Arc::ptr_eq(&self.relative_mosaic, &other.relative_mosaic)
    }
}

pub fn deduce_mosaics(slice_matrices: Vec<SliceMatrix>) -> Vec<WrappedMosaic> {
    slice_matrices.into_iter().map(WrappedMosaic::new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{CoordinatedPoint, WrappedCoordinateSystem};
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, WrappedRgbImage};
    use image::{ImageBuffer, Rgb};

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_vec_eq(actual: Vec3d, expected: Vec3d) {
        assert_float_eq(actual.x, expected.x);
        assert_float_eq(actual.y, expected.y);
        assert_float_eq(actual.z, expected.z);
    }

    fn global_coordinate_system() -> WrappedCoordinateSystem {
        WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        )
    }

    fn point(x: f64, y: f64) -> CoordinatedPoint {
        CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    fn slice(x1: f64, y: f64, x2: f64) -> Slice {
        Slice::new(point(x1, y), point(x2, y))
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(slice(x1, y, x2), line_number)
    }

    fn sample_slice_matrix(color: [u8; 3]) -> SliceMatrix {
        let image = WrappedRgbImage::new(ImageBuffer::from_pixel(32, 32, Rgb(color)));
        let mut matrix = SliceMatrix::new(image);
        matrix.add(SliceLine::new(1, vec![annotated_slice(1.0, 1.0, 3.0, 1)]));
        matrix.add(SliceLine::new(2, vec![annotated_slice(1.0, 2.0, 3.0, 2)]));
        matrix.add(SliceLine::new(3, vec![annotated_slice(1.0, 3.0, 3.0, 3)]));
        matrix
    }

    #[test]
    fn cached_relative_data_getters_return_constructor_values() {
        let coordinate_system = global_coordinate_system();
        let bounding_box = CoordinatedRectangle::new(point(0.1, 0.2), point(0.3, 0.4));
        let bounding_circle = CoordinatedCircle::new(point(0.2, 0.3), 0.15);
        let center_of_mass = CoordinatedPoint::new(coordinate_system, Vec3d::new(0.25, 0.35, 0.0));
        let cached = CachedRelativeData::new(
            bounding_box.clone(),
            bounding_circle.clone(),
            center_of_mass.clone(),
            0.45,
            Vec3d::new(10.0, 20.0, 30.0),
        );

        assert_float_eq(
            cached
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left()
                .x,
            0.1,
        );
        assert_float_eq(cached.get_bounding_circle().get_radius(), 0.15);
        assert_vec_eq(
            cached.get_center_of_mass().get_local_point(),
            Vec3d::new(0.25, 0.35, 0.0),
        );
        assert_float_eq(cached.get_area(), 0.45);
        assert_vec_eq(cached.get_average_color_vec(), Vec3d::new(10.0, 20.0, 30.0));
    }

    #[test]
    fn mosaic_methods_compute_expected_cached_geometry() {
        let mut mosaic = Mosaic::new(sample_slice_matrix([12, 34, 56]));

        assert_vec_eq(
            mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left(),
            Vec3d::new(1.0, 1.0, 0.0),
        );
        assert_vec_eq(
            mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_bottom_right(),
            Vec3d::new(3.0, 3.0, 0.0),
        );
        assert_float_eq(mosaic.get_bounding_circle().get_radius(), 1.0);
        assert_vec_eq(
            mosaic
                .get_center_of_mass()
                .convert_to(global_coordinate_system())
                .get_local_point(),
            Vec3d::new(2.0, 2.0, 0.0),
        );
        assert_float_eq(mosaic.get_area(), 9.0);
        assert!(mosaic.contains_point(point(2.0, 2.0)));
        assert!(!mosaic.contains_point(point(4.0, 4.0)));
        assert_vec_eq(
            mosaic
                .deduce_longest_distance_point(point(2.0, 2.0))
                .unwrap()
                .get_local_point(),
            Vec3d::new(1.0, 1.0, 0.0),
        );
        assert_vec_eq(mosaic.get_average_color(), Vec3d::new(12.0, 34.0, 56.0));
    }

    #[test]
    fn relative_mosaic_methods_map_values_into_relative_space() {
        let wrapped_mosaic = WrappedMosaic::new(sample_slice_matrix([90, 80, 70]));
        let absolute_rectangle =
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 20.0, 0.0));
        let mut relative_mosaic =
            RelativeMosaic::new(wrapped_mosaic.clone(), absolute_rectangle.clone());

        assert!(relative_mosaic.get_mosaic().get_area() == wrapped_mosaic.get_area());
        assert!(relative_mosaic.get_absolute_rectangle() == absolute_rectangle);
        assert_vec_eq(
            relative_mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left(),
            Vec3d::new(0.09090909090909091, 0.047619047619047616, 0.0),
        );
        assert_vec_eq(
            relative_mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_bottom_right(),
            Vec3d::new(0.36363636363636365, 0.19047619047619047, 0.0),
        );
        assert_vec_eq(
            relative_mosaic
                .get_center_of_mass()
                .convert_to(global_coordinate_system())
                .get_local_point(),
            Vec3d::new(0.2, 0.1, 0.0),
        );
        assert_float_eq(relative_mosaic.get_bounding_circle().get_radius(), 0.1);
        assert_float_eq(relative_mosaic.get_area(), 0.045);
        assert_vec_eq(
            relative_mosaic.get_average_color(),
            Vec3d::new(90.0, 80.0, 70.0),
        );
    }

    #[test]
    fn wrapped_mosaic_and_relative_mosaic_methods_forward_to_inner_types() {
        let wrapped_mosaic = WrappedMosaic::new(sample_slice_matrix([1, 2, 3]));
        let wrapped_relative = WrappedRelativeMosaic::new(
            wrapped_mosaic.clone(),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 20.0, 0.0)),
        );

        assert_float_eq(wrapped_mosaic.get_area(), 9.0);
        assert!(wrapped_mosaic.contains_point(point(2.0, 2.0)));
        assert_eq!(wrapped_mosaic.get_slice_matrix().get_slice_lines().len(), 3);
        assert_vec_eq(
            wrapped_mosaic.get_average_color(),
            Vec3d::new(1.0, 2.0, 3.0),
        );
        assert_float_eq(wrapped_relative.get_area(), 0.045);
        assert_vec_eq(
            wrapped_relative.get_average_color(),
            Vec3d::new(1.0, 2.0, 3.0),
        );
        assert!(wrapped_relative.get_mosaic().get_area() == wrapped_mosaic.get_area());
    }

    #[test]
    fn wrapped_relative_mosaic_identity_and_deduce_mosaics_behave_as_expected() {
        let first_matrix = sample_slice_matrix([5, 6, 7]);
        let second_matrix = sample_slice_matrix([8, 9, 10]);
        let mosaics = deduce_mosaics(vec![first_matrix.clone(), second_matrix.clone()]);
        let first_wrapped = WrappedRelativeMosaic::new(
            mosaics[0].clone(),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 20.0, 0.0)),
        );
        let same_wrapper = first_wrapped.clone();
        let distinct_wrapper = WrappedRelativeMosaic::new(
            mosaics[0].clone(),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 20.0, 0.0)),
        );

        assert_eq!(mosaics.len(), 2);
        assert!(first_wrapped.shares_identity_with(&same_wrapper));
        assert!(!first_wrapped.shares_identity_with(&distinct_wrapper));
    }
}
