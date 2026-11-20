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
}