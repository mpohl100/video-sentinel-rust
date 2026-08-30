use std::collections::BTreeMap;

use crate::mosaics::WrappedRelativeMosaic;
use crate::slices::{Rectangle, WrappedRelativeRectangle};

pub struct BucketedMosaicsPerSection {
    region: WrappedRelativeRectangle,
    bucket: BTreeMap<i64, Vec<WrappedRelativeMosaic>>,
    delta: f64,
}

impl BucketedMosaicsPerSection {
    pub fn new(region: WrappedRelativeRectangle, delta: f64) -> Self {
        BucketedMosaicsPerSection {
            region,
            bucket: BTreeMap::new(),
            delta,
        }
    }

    pub fn add_mosaic(&mut self, mosaic: WrappedRelativeMosaic) {
        let bounding_box =
            Rectangle::new_from_math_rectangle(mosaic.get_bounding_box().to_global_rectangle());
        if self.region.overlaps(&bounding_box) {
            self.bucket
                .entry(self.get_bucket_key(&mosaic))
                .or_default()
                .push(mosaic);
        }
    }

    fn get_bucket_key(&self, mosaic: &WrappedRelativeMosaic) -> i64 {
        let bounding_circle_area = mosaic.get_bounding_circle().get_area();
        let mosaic_area = mosaic.get_area();
        if bounding_circle_area == 0.0 {
            0
        } else {
            ((mosaic_area / bounding_circle_area) / self.delta).floor() as i64
        }
    }

    pub fn get_potentially_similar_mosaics(
        &self,
        mosaic: &WrappedRelativeMosaic,
    ) -> Vec<WrappedRelativeMosaic> {
        let bucket_key = self.get_bucket_key(mosaic);
        let mut similar_mosaics = Vec::new();
        for key in bucket_key - 1..=bucket_key + 1 {
            if let Some(mosaics) = self.bucket.get(&key) {
                similar_mosaics.extend(mosaics.clone());
            }
        }
        similar_mosaics
    }
}

pub struct BucketedMosaics {
    sections: Vec<BucketedMosaicsPerSection>,
}

impl BucketedMosaics {
    fn push_unique(
        similar_mosaics: &mut Vec<WrappedRelativeMosaic>,
        candidate: WrappedRelativeMosaic,
    ) {
        if !similar_mosaics
            .iter()
            .any(|existing| existing.shares_identity_with(&candidate))
        {
            similar_mosaics.push(candidate);
        }
    }

    pub fn new(regions: Vec<WrappedRelativeRectangle>, delta: f64) -> Self {
        let sections = regions
            .into_iter()
            .map(|region| BucketedMosaicsPerSection::new(region, delta))
            .collect();
        BucketedMosaics { sections }
    }

    pub fn add_mosaic(&mut self, mosaic: WrappedRelativeMosaic) {
        for section in &mut self.sections {
            section.add_mosaic(mosaic.clone());
        }
    }

    pub fn get_potentially_similar_mosaics(
        &self,
        mosaic: &WrappedRelativeMosaic,
    ) -> Vec<WrappedRelativeMosaic> {
        let mut similar_mosaics: Vec<WrappedRelativeMosaic> = Vec::new();
        for section in self.get_overlapping_sections(Rectangle::new_from_math_rectangle(
            mosaic.get_bounding_box().to_global_rectangle(),
        )) {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                Self::push_unique(&mut similar_mosaics, candidate);
            }
        }
        similar_mosaics
    }

    pub fn get_all_similar_mosaics(
        &self,
        mosaic: &WrappedRelativeMosaic,
    ) -> Vec<WrappedRelativeMosaic> {
        let mut similar_mosaics: Vec<WrappedRelativeMosaic> = Vec::new();
        for section in &self.sections {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                Self::push_unique(&mut similar_mosaics, candidate);
            }
        }
        similar_mosaics
    }

    pub fn get_similar_mosaics_from_rectangle(
        &self,
        mosaic: &WrappedRelativeMosaic,
        region: WrappedRelativeRectangle,
    ) -> Vec<WrappedRelativeMosaic> {
        let mut similar_mosaics: Vec<WrappedRelativeMosaic> = Vec::new();
        for section in self.get_overlapping_sections(region.to_rectangle()) {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                Self::push_unique(&mut similar_mosaics, candidate);
            }
        }
        similar_mosaics
    }

    fn get_overlapping_sections(&self, bounding_box: Rectangle) -> Vec<&BucketedMosaicsPerSection> {
        self.sections
            .iter()
            .filter(|section| section.region.overlaps(&bounding_box))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Rectangle as MathRectangle;
    use crate::mosaics::WrappedMosaic;
    use crate::slices::{AnnotatedSlice, RelativeRectangle, Slice, SliceLine, SliceMatrix, WrappedRgbImage};
    use image::{ImageBuffer, Rgb};
    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn global_region(top_left: Vec3d, bottom_right: Vec3d) -> WrappedRelativeRectangle {
        WrappedRelativeRectangle::new_from_rectangles(
            Rectangle::new(
                Vec3d::new(top_left.x * 100.0, top_left.y * 100.0, 0.0),
                Vec3d::new(bottom_right.x * 100.0 - 1.0, bottom_right.y * 100.0 - 1.0, 0.0),
            ),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(99.0, 99.0, 0.0)),
        )
    }

    fn solid_image() -> WrappedRgbImage {
        WrappedRgbImage::new(ImageBuffer::from_pixel(64, 64, Rgb([20, 40, 60])))
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(
            Slice::new(
                crate::math::CoordinatedPoint::new(
                    crate::math::WrappedCoordinateSystem::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(1.0, 0.0, 0.0),
                        Vec3d::new(0.0, 1.0, 0.0),
                    ),
                    Vec3d::new(x1, y, 0.0),
                ),
                crate::math::CoordinatedPoint::new(
                    crate::math::WrappedCoordinateSystem::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(1.0, 0.0, 0.0),
                        Vec3d::new(0.0, 1.0, 0.0),
                    ),
                    Vec3d::new(x2, y, 0.0),
                ),
            ),
            line_number,
        )
    }

    fn mosaic_from_lines(lines: &[(usize, &[(f64, f64)])]) -> WrappedMosaic {
        let mut matrix = SliceMatrix::new(solid_image());
        for (line_number, ranges) in lines {
            let slices = ranges
                .iter()
                .map(|(start, end)| annotated_slice(*start, *line_number as f64, *end, *line_number))
                .collect();
            matrix.add(SliceLine::new(*line_number, slices));
        }
        WrappedMosaic::new(matrix)
    }

    fn relative_mosaic(lines: &[(usize, &[(f64, f64)])]) -> WrappedRelativeMosaic {
        WrappedRelativeMosaic::new(
            mosaic_from_lines(lines),
            MathRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 10.0, 0.0)),
        )
    }

    fn bounding_box_signature(mosaic: &WrappedRelativeMosaic) -> (Vec3d, Vec3d) {
        let rectangle = mosaic.get_bounding_box().to_global_rectangle();
        (rectangle.get_top_left(), rectangle.get_bottom_right())
    }

    fn assert_signature(mosaic: &WrappedRelativeMosaic, expected_top_left: Vec3d, expected_bottom_right: Vec3d) {
        let (top_left, bottom_right) = bounding_box_signature(mosaic);
        assert_float_eq(top_left.x, expected_top_left.x);
        assert_float_eq(top_left.y, expected_top_left.y);
        assert_float_eq(bottom_right.x, expected_bottom_right.x);
        assert_float_eq(bottom_right.y, expected_bottom_right.y);
    }

    #[test]
    fn bucketed_mosaics_per_section_new_initializes_empty_bucket() {
        let region = global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));
        let section = BucketedMosaicsPerSection::new(region.clone(), 0.5);

        assert!(section.bucket.is_empty());
        assert_float_eq(section.delta, 0.5);
        assert!(section.region.overlaps(&region.to_rectangle()));
    }

    #[test]
    fn bucket_key_returns_zero_for_zero_radius_and_expected_ratio_for_nonzero_radius() {
        let region = global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));
        let section = BucketedMosaicsPerSection::new(region, 0.5);
        let single_point = relative_mosaic(&[(0, &[(0.0, 0.0)])]);
        let vertical_pair = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let three_rows = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)]), (2, &[(0.0, 0.0)])]);

        assert_eq!(section.get_bucket_key(&single_point), 0);
        assert_eq!(section.get_bucket_key(&vertical_pair), 5);
        assert_eq!(section.get_bucket_key(&three_rows), 1);
    }

    #[test]
    fn add_mosaic_only_stores_overlapping_mosaics_in_matching_bucket() {
        let region = global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0));
        let mut section = BucketedMosaicsPerSection::new(region, 0.5);
        let inside = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let outside = WrappedRelativeMosaic::new(
            mosaic_from_lines(&[(9, &[(9.0, 9.0)]), (10, &[(9.0, 9.0)])]),
            MathRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 10.0, 0.0)),
        );

        section.add_mosaic(inside.clone());
        section.add_mosaic(outside);

        assert_eq!(section.bucket.len(), 1);
        assert_eq!(section.bucket.get(&5).unwrap().len(), 1);
        assert_signature(
            &section.bucket.get(&5).unwrap()[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
    }

    #[test]
    fn get_potentially_similar_mosaics_returns_neighboring_buckets_only() {
        let region = global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));
        let mut section = BucketedMosaicsPerSection::new(region, 0.5);
        let bucket_zero = relative_mosaic(&[(0, &[(0.0, 0.0)])]);
        let bucket_one = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)]), (2, &[(0.0, 0.0)])]);
        let bucket_five = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);

        section.add_mosaic(bucket_zero.clone());
        section.add_mosaic(bucket_one.clone());
        section.add_mosaic(bucket_five.clone());

        let similar_to_bucket_one = section.get_potentially_similar_mosaics(&bucket_one);
        let similar_to_bucket_five = section.get_potentially_similar_mosaics(&bucket_five);

        assert_eq!(similar_to_bucket_one.len(), 2);
        assert_signature(
            &similar_to_bucket_one[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.09090909090909091, 0.0),
        );
        assert_signature(
            &similar_to_bucket_one[1],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.2727272727272727, 0.0),
        );
        assert_eq!(similar_to_bucket_five.len(), 1);
        assert_signature(
            &similar_to_bucket_five[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
    }

    #[test]
    fn bucketed_mosaics_new_creates_one_section_per_region() {
        let bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.4, 0.4, 0.0)),
                global_region(Vec3d::new(0.5, 0.5, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
            ],
            0.5,
        );

        assert_eq!(bucketed.sections.len(), 2);
    }

    #[test]
    fn add_mosaic_places_one_mosaic_into_each_overlapping_section() {
        let mosaic = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let mut bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.6, 0.6, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
            ],
            0.5,
        );

        bucketed.add_mosaic(mosaic);

        assert_eq!(bucketed.sections[0].bucket.get(&5).unwrap().len(), 1);
        assert_eq!(bucketed.sections[1].bucket.get(&5).unwrap().len(), 1);
        assert!(bucketed.sections[2].bucket.is_empty());
    }

    #[test]
    fn get_potentially_similar_mosaics_reads_only_overlapping_sections() {
        let query = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let far = relative_mosaic(&[(8, &[(8.0, 8.0)]), (9, &[(8.0, 8.0)])]);
        let mut bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.7, 0.7, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
            ],
            0.5,
        );

        bucketed.add_mosaic(query.clone());
        bucketed.add_mosaic(far);

        let similar = bucketed.get_potentially_similar_mosaics(&query);

        assert_eq!(similar.len(), 1);
        assert_signature(
            &similar[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
    }

    #[test]
    fn get_all_similar_mosaics_deduplicates_results_across_overlapping_sections() {
        let query = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let mut bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
            ],
            0.5,
        );

        bucketed.add_mosaic(query.clone());

        let similar = bucketed.get_all_similar_mosaics(&query);

        assert_eq!(similar.len(), 1);
        assert_signature(
            &similar[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
    }

    #[test]
    fn get_similar_mosaics_from_rectangle_filters_sections_by_requested_region() {
        let query = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let far = relative_mosaic(&[(8, &[(8.0, 8.0)]), (9, &[(8.0, 8.0)])]);
        let mut bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.7, 0.7, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
            ],
            0.5,
        );

        bucketed.add_mosaic(query.clone());
        bucketed.add_mosaic(far);

        let near_only = bucketed.get_similar_mosaics_from_rectangle(
            &query,
            global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
        );
        let far_only = bucketed.get_similar_mosaics_from_rectangle(
            &query,
            global_region(Vec3d::new(0.7, 0.7, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
        );

        assert_eq!(near_only.len(), 1);
        assert_eq!(far_only.len(), 1);
        assert_signature(
            &near_only[0],
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
        assert_signature(
            &far_only[0],
            Vec3d::new(0.7272727272727273, 0.7272727272727273, 0.0),
            Vec3d::new(0.8181818181818182, 0.9090909090909091, 0.0),
        );
    }

    #[test]
    fn get_overlapping_sections_returns_only_regions_that_overlap_bounding_box() {
        let bucketed = BucketedMosaics::new(
            vec![
                global_region(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.2, 0.2, 0.0)),
                global_region(Vec3d::new(0.1, 0.1, 0.0), Vec3d::new(0.4, 0.4, 0.0)),
                global_region(Vec3d::new(0.7, 0.7, 0.0), Vec3d::new(1.0, 1.0, 0.0)),
            ],
            0.5,
        );
        let query_box = Rectangle::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.15, 0.15, 0.0),
        );

        let overlapping_sections = bucketed.get_overlapping_sections(query_box);

        assert_eq!(overlapping_sections.len(), 2);
        assert!(overlapping_sections[0]
            .region
            .overlaps(&Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.15, 0.15, 0.0))));
        assert!(overlapping_sections[1]
            .region
            .overlaps(&Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.15, 0.15, 0.0))));
    }

    #[test]
    fn bucketed_mosaic_tests_cover_relative_mosaic_assumptions_used_by_bucketing() {
        let mosaic = relative_mosaic(&[(0, &[(0.0, 0.0)]), (1, &[(0.0, 0.0)])]);
        let bounding_circle = mosaic.get_bounding_circle();

        assert_signature(
            &mosaic,
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.09090909090909091, 0.18181818181818182, 0.0),
        );
        assert_float_eq(mosaic.get_area(), 0.02);
        assert_float_eq(bounding_circle.get_center().get_x(), 0.0);
        assert_float_eq(bounding_circle.get_center().get_y(), 0.05);
        assert_float_eq(bounding_circle.get_radius(), 0.05);
        assert_float_eq(bounding_circle.get_area(), 0.007853981633974483);
        assert_float_eq(mosaic.get_absolute_rectangle().get_area(), 100.0);
        assert_float_eq(mosaic.get_mosaic().get_area(), 2.0);
    }

    #[test]
    fn constructing_regions_through_relative_rectangles_preserves_requested_bounds() {
        let region = global_region(Vec3d::new(0.25, 0.5, 0.0), Vec3d::new(0.75, 0.9, 0.0));
        let direct = WrappedRelativeRectangle::new_from_rectangles(
            Rectangle::new(Vec3d::new(25.0, 50.0, 0.0), Vec3d::new(74.0, 89.0, 0.0)),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(99.0, 99.0, 0.0)),
        );
        let manual = WrappedRelativeRectangle::new(RelativeRectangle::new_from_rectangles(
            Rectangle::new(Vec3d::new(25.0, 50.0, 0.0), Vec3d::new(74.0, 89.0, 0.0)),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(99.0, 99.0, 0.0)),
        ));

        assert_float_eq(region.to_rectangle().get_top_left().x, 0.25);
        assert_float_eq(region.to_rectangle().get_bottom_right().y, 0.9);
        assert_float_eq(direct.to_rectangle().get_top_left().x, 0.25);
        assert_float_eq(manual.to_rectangle().get_bottom_right().x, 0.75);
    }
}
