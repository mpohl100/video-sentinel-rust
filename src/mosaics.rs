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
        let unit_rectangle =
            SliceRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 0.0));
        let relative_rectangle =
            RelativeRectangle::new_from_rectangles(bounding_box, absolute_rectangle);
        let relative_bounding_box = relative_rectangle.multiply_with_rectangle(unit_rectangle);
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
}

pub fn deduce_mosaics(slice_matrices: Vec<SliceMatrix>) -> Vec<WrappedMosaic> {
    slice_matrices.into_iter().map(WrappedMosaic::new).collect()
}
