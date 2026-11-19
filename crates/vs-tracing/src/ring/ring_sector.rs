use rs_math3d::FloatVector;
use vs_math::math::AnonymizedCoordinateSystem;
use vs_math::math::CoordinatedCircle;
use vs_math::math::CoordinatedLine;
use vs_math::math::CoordinatedPoint;
use vs_math::math::CoordinatedRegionedAngle;
use vs_math::math::CoordinatedPartialCircle;
use vs_math::math::PolarCoordinates;

use rs_math3d::Vec3d;

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
        let nb_points_with_positive_x = [top_left.clone(), bottom_right.clone(), top_right.clone(), bottom_left.clone()]
            .iter()
            .filter(|p| p.get_x() > 0.0)
            .count();

        if nb_points_with_positive_x == 0 || nb_points_with_positive_x == 4 {
            return None;
        }

        let coordinated_start_angle = self.top_left.get_angle();
        let coordinated_end_angle = self.bottom_right.get_angle();
        // calculate the intersection of the top partial circle with the x-axis as an Option<f64>
        let top_circle = CoordinatedCircle::new(CoordinatedPoint::new(self.top_left.get_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)), self.top_left.get_radius());
        let x_axis_line = CoordinatedLine::new(
            CoordinatedPoint::new(coordinate_system.duplicate(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(coordinate_system.duplicate(), Vec3d::new(10000.0, 0.0, 0.0)),
        );
        let top_partial_circle = CoordinatedPartialCircle::new(top_circle.clone(), coordinated_start_angle.clone(), coordinated_end_angle.clone());
        let intersection_points_top_partial_circle = top_partial_circle.get_intersection_points(&x_axis_line);
        // calculate the intersection of the bottom partial circle with the x-axis as an Option<f64>
        let bottom_circle = CoordinatedCircle::new(CoordinatedPoint::new(self.bottom_right.get_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)), self.bottom_right.get_radius());
        let bottom_partial_circle = CoordinatedPartialCircle::new(bottom_circle.clone(), coordinated_start_angle, coordinated_end_angle);
        let intersection_points_bottom_partial_circle = bottom_partial_circle.get_intersection_points(&x_axis_line);

        // calculate the intersection of the left line of the ring sector with the x-axis as an Option<f64>
        let left_line = CoordinatedLine::new(top_left.clone(), bottom_left.clone());
        let intersection_point_left = left_line.get_intersection_point(&x_axis_line);
        // calculate the intersection of the right line of the ring sector with the x-axis as an Option<f64>
        let right_line = CoordinatedLine::new(top_right.clone(), bottom_right.clone());
        let intersection_point_right = right_line.get_intersection_point(&x_axis_line);

        // two of the above calculations should contains some coordinate and return the coordinated line of those 
        // concatenate all intersection points into a single vector
        let mut all_intersection_points = Vec::new();
        all_intersection_points.extend(intersection_points_top_partial_circle);
        all_intersection_points.extend(intersection_points_bottom_partial_circle);
        if let Some(point) = intersection_point_left {
            all_intersection_points.push(point);
        }
        if let Some(point) = intersection_point_right {
            all_intersection_points.push(point);
        }

        if all_intersection_points.len() < 2 {
            return None;
        }
        else if all_intersection_points.len() > 2 {
            // if there are more than 2 intersection points, return the line connecting the first and last points
            // calculate the magintude of the vectors between all intersection points and return the line connecting the two points with the largest distance
            let mut max_distance = 0.0;
            let mut line = None;
            for i in 0..all_intersection_points.len() {
                for j in i+1..all_intersection_points.len() {
                    let distance = (all_intersection_points[i].get_local_point() - all_intersection_points[j].get_local_point()).length();
                    if distance > max_distance {
                        max_distance = distance;
                        line = Some(CoordinatedLine::new(
                            all_intersection_points[i].clone(),
                            all_intersection_points[j].clone(),
                        ));
                    }
                }
            }
            return line;
        }
        else if all_intersection_points.len() == 2 {
            return Some(CoordinatedLine::new(
                all_intersection_points[0].clone(),
                all_intersection_points[1].clone(),
            ));
        }
        None
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
