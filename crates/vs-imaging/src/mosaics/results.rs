#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Controls whether enriched outputs use source mosaic coordinates (`Absolute`)
/// or normalized coordinates derived from `WrappedRelativeMosaic` (`Relative`).
pub enum Results {
    Absolute,
    Relative,
}

#[cfg(test)]
mod tests {

    use crate::math::CoordinateSystem;
    use crate::mosaics::*;
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, WrappedRgbImage};
    use crate::{
        math::{
            AnonymizedCoordinateSystem, CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle,
            Rectangle,
        },
        slices::SliceMatrix,
    };
    use rs_math3d::Vec3d;

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

    fn global_coordinate_system() -> AnonymizedCoordinateSystem {
        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ))
    }

    fn point(x: f64, y: f64) -> CoordinatedPoint {
        CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    fn slice(x1: f64, y: f64, x2: f64) -> Slice {
        Slice::new(&point(x1, y), &point(x2, y))
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(slice(x1, y, x2), line_number)
    }

    fn sample_slice_matrix(color: [u8; 3]) -> SliceMatrix {
        let image = WrappedRgbImage::new_with_color(32, 32, color);
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
        assert!(mosaic.contains_point(&point(2.0, 2.0)));
        assert!(!mosaic.contains_point(&point(4.0, 4.0)));
        assert_vec_eq(
            mosaic
                .deduce_longest_distance_point(&point(2.0, 2.0))
                .unwrap()
                .get_local_point(),
            Vec3d::new(1.0, 4.0, 0.0),
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

        assert_float_eq(
            relative_mosaic.get_mosaic().get_area(),
            wrapped_mosaic.get_area(),
        );
        assert!(relative_mosaic.get_absolute_rectangle() == absolute_rectangle);
        assert_vec_eq(
            relative_mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left(),
            Vec3d::new(0.090_909_090_909_090_91, 0.047_619_047_619_047_616, 0.0),
        );
        assert_vec_eq(
            relative_mosaic
                .get_bounding_box()
                .to_global_rectangle()
                .get_bottom_right(),
            Vec3d::new(0.363_636_363_636_363_65, 0.190_476_190_476_190_47, 0.0),
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
        assert!(wrapped_mosaic.contains_point(&point(2.0, 2.0)));
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
        assert_float_eq(
            wrapped_relative.get_mosaic().get_area(),
            wrapped_mosaic.get_area(),
        );
    }

    #[test]
    fn wrapped_relative_mosaic_identity_and_deduce_mosaics_behave_as_expected() {
        let first_matrix = sample_slice_matrix([5, 6, 7]);
        let second_matrix = sample_slice_matrix([8, 9, 10]);
        let math_rectangle =
            crate::math::Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 20.0, 0.0));
        let mosaics = deduce_mosaics(
            vec![first_matrix.clone(), second_matrix.clone()],
            &math_rectangle,
        );
        let first_wrapped = mosaics[0].clone();
        let same_wrapper = first_wrapped.clone();
        let distinct_wrapper = mosaics[0].clone();

        assert_eq!(mosaics.len(), 2);
        assert!(first_wrapped.shares_identity_with(&same_wrapper));
        assert!(first_wrapped.shares_identity_with(&distinct_wrapper));
    }
}
