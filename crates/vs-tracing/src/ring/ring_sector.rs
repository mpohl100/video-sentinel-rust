use vs_math::math::AnonymizedCoordinateSystem;
use vs_math::math::CoordinatedCircle;
use vs_math::math::CoordinatedLine;
use vs_math::math::CoordinatedPoint;
use vs_math::math::CoordinatedRegionedAngle;
use vs_math::math::PolarCoordinates;
use vs_math::math::Vec3d;

#[derive(Clone)]
pub struct RingSector {
    top_left: PolarCoordinates,
    bottom_right: PolarCoordinates,
}

impl RingSector {
    pub fn new_from_radiuses_and_angles(
        radiuses: (f64, f64),
        angles: (CoordinatedRegionedAngle, CoordinatedRegionedAngle),
    ) -> Self {
        let bigger_radius = radiuses.0.max(radiuses.1);
        let smaller_radius = radiuses.0.min(radiuses.1);
        let converted_angles = (
            angles.0.clone(),
            angles.1.convert_to(angles.0.get_coordinate_system()),
        );
        let (bigger_angle, smaller_angle) =
            if converted_angles.0.get_angle_degrees() > converted_angles.1.get_angle_degrees() {
                (converted_angles.0, converted_angles.1)
            } else {
                (converted_angles.1, converted_angles.0)
            };
        Self {
            top_left: PolarCoordinates::new(bigger_radius, bigger_angle),
            bottom_right: PolarCoordinates::new(smaller_radius, smaller_angle),
        }
    }

    pub fn get_intersection_with_x_axis(
        &self,
        coordinate_system: AnonymizedCoordinateSystem,
    ) -> Option<CoordinatedLine> {
        let top_left = self
            .get_top_left_cartesian()
            .convert_to(coordinate_system.duplicate());
        let bottom_right = self
            .get_bottom_right_cartesian()
            .convert_to(coordinate_system.duplicate());
        let top_right = self
            .get_top_right_cartesian()
            .convert_to(coordinate_system.duplicate());
        let bottom_left = self
            .get_bottom_left_cartesian()
            .convert_to(coordinate_system.duplicate());
        let nb_points_with_positive_x = [top_left, bottom_right, top_right, bottom_left]
            .iter()
            .filter(|p| p.get_x() > 0.0)
            .count();

        if nb_points_with_positive_x == 0 || nb_points_with_positive_x == 4 {
            return None;
        }
        // calculate the intersection of the top partial circle with the x-axis as an Option<f64>
        let top_circle = CoordinatedCircle::new(CoordinatedPoint::new(self.top_left.get_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)), self.top_left.get_radius());
        let x_axis_line = CoordinatedLine::new(
            CoordinatedPoint::new(coordinate_system.duplicate(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(coordinate_system.duplicate(), Vec3d::new(10000.0, 0.0, 0.0)),
        );
        let intersection_line_top_circle = top_circle.get_intersection_line(&x_axis_line);
        // calculate the intersection of the bottom partial circle with the x-axis as an Option<f64>
        let bottom_circle = CoordinatedCircle::new(CoordinatedPoint::new(self.bottom_right.get_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)), self.bottom_right.get_radius());
        let intersection_line_bottom_circle = bottom_circle.get_intersection_line(&x_axis_line);

        // calculate the intersection of the left line of the ring sector with the x-axis as an Option<f64>
        let left_line = CoordinatedLine::new(top_left, bottom_left);
        let intersection_line_left = left_line.get_intersection_point(&x_axis_line);
        // calculate the intersection of the right line of the ring sector with the x-axis as an Option<f64>
        let right_line = CoordinatedLine::new(top_right, bottom_right);
        let intersection_line_right = right_line.get_intersection_point(&x_axis_line);

        // two of the above calculations should contains some coordinate and return the coordinated line of those 

        Some(CoordinatedLine::new(top_left, bottom_right))
    }

    fn get_top_left_cartesian(&self) -> CoordinatedPoint {
        self.top_left.to_cartesian()
    }

    fn get_top_right_cartesian(&self) -> CoordinatedPoint {
        PolarCoordinates::new(self.top_left.get_radius(), self.bottom_right.get_angle())
            .to_cartesian()
    }

    fn get_bottom_right_cartesian(&self) -> CoordinatedPoint {
        self.bottom_right.to_cartesian()
    }

    fn get_bottom_left_cartesian(&self) -> CoordinatedPoint {
        PolarCoordinates::new(self.bottom_right.get_radius(), self.top_left.get_angle())
            .to_cartesian()
    }
}
