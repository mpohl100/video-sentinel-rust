use vs_math::math::PolarCoordinates;
use vs_math::math::CoordinatedRegionedAngle;



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
        let converted_angles = (angles.0.clone(), angles.1.convert_to(angles.0.get_coordinate_system()));
        let (bigger_angle, smaller_angle) = if converted_angles.0.get_angle_degrees() > converted_angles.1.get_angle_degrees() {
            (converted_angles.0, converted_angles.1)
        } else {
            (converted_angles.1, converted_angles.0)
        };
        Self {
            top_left: PolarCoordinates::new(bigger_radius, bigger_angle),
            bottom_right: PolarCoordinates::new(smaller_radius, smaller_angle),
        }
    }
}