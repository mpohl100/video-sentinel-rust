use crate::mosaics::WrappedMosaic;
use crate::traces::{Trace, TraceParams};

#[derive(Clone)]
pub struct TracedMosaic {
    mosaic: WrappedMosaic,
    trace: Trace,
}

impl TracedMosaic {
    pub fn new(mosaic: WrappedMosaic, trace_params: TraceParams) -> Self {
        let trace = Trace::new_from_mosaic(mosaic.clone(), trace_params);
        TracedMosaic { mosaic, trace }
    }

    pub fn get_mosaic(&self) -> &WrappedMosaic {
        &self.mosaic
    }

    pub fn get_trace(&self) -> &Trace {
        &self.trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{CoordinatedPoint, WrappedCoordinateSystem};
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, SliceMatrix, WrappedRgbImage};
    use image::{ImageBuffer, Rgb};
    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
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

    fn sample_mosaic() -> WrappedMosaic {
        let image = WrappedRgbImage::new(ImageBuffer::from_pixel(16, 16, Rgb([200, 100, 50])));
        let mut matrix = SliceMatrix::new(image);
        matrix.add(SliceLine::new(
            2,
            vec![AnnotatedSlice::new(
                Slice::new(point(2.0, 2.0), point(4.0, 2.0)),
                2,
            )],
        ));
        matrix.add(SliceLine::new(
            3,
            vec![AnnotatedSlice::new(
                Slice::new(point(2.0, 3.0), point(4.0, 3.0)),
                3,
            )],
        ));
        matrix.add(SliceLine::new(
            4,
            vec![AnnotatedSlice::new(
                Slice::new(point(2.0, 4.0), point(4.0, 4.0)),
                4,
            )],
        ));
        WrappedMosaic::new(matrix)
    }

    #[test]
    fn traced_mosaic_new_builds_trace_and_exposes_original_mosaic() {
        let mosaic = sample_mosaic();
        let trace_params = TraceParams::new(12, 0.2);
        let direct_trace = Trace::new_from_mosaic(mosaic.clone(), trace_params.clone());
        let traced_mosaic = TracedMosaic::new(mosaic.clone(), trace_params);

        assert_float_eq(traced_mosaic.get_mosaic().get_area(), mosaic.get_area());
        assert_float_eq(
            traced_mosaic.get_trace().compare_with(1.0, &direct_trace),
            1.0333333333333334,
        );
    }
}
