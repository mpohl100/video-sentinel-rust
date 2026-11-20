use vs_math::math::CoordinatedRegionedAngle;

use crate::math::CoordinatedLine;
use crate::ring::ring_sector::RingSector;

#[derive(Clone)]
pub struct Pizza {
    ring_sectors: Vec<RingSector>
}

impl Pizza {
    pub fn new(ring_sectors: Vec<RingSector>) -> Self {
        Self { ring_sectors }
    }

    pub fn add_ring_sector(&mut self, ring_sector: RingSector) {
        self.ring_sectors.push(ring_sector);
    }

    pub fn get_ring_sectors(&self) -> &Vec<RingSector> {
        &self.ring_sectors
    }

    pub fn get_area(&self) -> f64 {
        self.ring_sectors.iter().map(|rs| rs.get_area()).sum()
    }

    pub fn get_contained_lines(&self, coordinated_regioned_angle: &CoordinatedRegionedAngle, radius: f64) -> Vec<CoordinatedLine> {
        let mut contained_lines = Vec::new();
        for ring_sector in &self.ring_sectors {
            let mut coordinate_system = coordinated_regioned_angle.get_coordinate_system().duplicate();
            coordinate_system.rotate(coordinated_regioned_angle.get_regioned_angle().inverted());
            contained_lines.extend(ring_sector.get_intersection_with_x_axis(coordinate_system, radius));
        }
        contained_lines
    }
}