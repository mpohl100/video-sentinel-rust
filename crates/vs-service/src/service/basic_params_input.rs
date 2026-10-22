#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy)]
pub struct BasicParamsInput {
    pub do_grayscale: bool,
    pub gradient_threshold: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mosaics::Results;

    use crate::service::*;

    use crate::slices::BasicParams;
    use crate::slices::Color;
    use crate::slices::Rectangle;
    use crate::slices::WrappedRgbImage;

    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn basic_params_input() -> BasicParamsInput {
        BasicParamsInput {
            do_grayscale: false,
            gradient_threshold: 15,
        }
    }

    fn tile_params_input() -> TileParamsInput {
        TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        }
    }

    fn trace_params_input() -> TraceParamsInput {
        TraceParamsInput { num_skeleton: 12 }
    }

    fn eye_params_input() -> EyeParamsInput {
        EyeParamsInput {
            tile_params: tile_params_input(),
            bucket_delta: 0.25,
            trace_params: trace_params_input(),
            target_similarity: 0.9,
        }
    }

    fn object_detection_params_input() -> ObjectDetectionParamsInput {
        ObjectDetectionParamsInput {
            tile_params: tile_params_input(),
            bucket_delta: 0.25,
            trace_params: trace_params_input(),
            target_similarity: 0.9,
        }
    }

    fn solid_image(color: [u8; 3]) -> WrappedRgbImage {
        WrappedRgbImage::new_with_color(4, 4, color)
    }

    fn larger_solid_image(color: [u8; 3]) -> WrappedRgbImage {
        WrappedRgbImage::new_with_color(16, 16, color)
    }

    fn object_ascii_art() -> String {
        [
            "#######", "#######", "#######", "#######", "#######", "#######", "#######",
        ]
        .join("\n")
    }

    fn empty_object_ascii_art() -> String {
        ["##", "##"].join("\n")
    }

    fn tiny_surrounding_rectangle() -> Rectangle {
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.0, 0.0))
    }

    fn object_image() -> WrappedRgbImage {
        WrappedRgbImage::new_from_ascii_art(object_ascii_art().as_str())
    }

    #[test]
    fn service_creates_sessions_and_exposes_typed_getters() {
        let mut service = Service::new();

        assert!(matches!(
            service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute),
            CreateOrdinarySessionResult::Success
        ));
        assert!(matches!(
            service.create_eye_session(
                "eye",
                basic_params_input(),
                eye_params_input(),
                Results::Relative
            ),
            CreateEyeSessionResult::Success
        ));
        assert!(matches!(
            service.create_object_session(
                "object",
                basic_params_input(),
                object_detection_params_input(),
                Results::Absolute,
            ),
            CreateObjectSessionResult::Success
        ));

        assert!(service.get_ordinary_session("ordinary").is_some());
        assert!(service.get_eye_session("eye").is_some());
        assert!(service.get_object_session("object").is_some());
        assert!(matches!(
            service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute),
            CreateOrdinarySessionResult::SessionAlreadyExists
        ));
        assert!(matches!(
            service.create_eye_session(
                "eye",
                basic_params_input(),
                eye_params_input(),
                Results::Relative
            ),
            CreateEyeSessionResult::SessionAlreadyExists
        ));
        assert!(matches!(
            service.create_object_session(
                "object",
                basic_params_input(),
                object_detection_params_input(),
                Results::Absolute,
            ),
            CreateObjectSessionResult::SessionAlreadyExists
        ));
    }

    #[test]
    fn service_updates_supported_sessions_and_rejects_wrong_types() {
        let mut service = Service::new();
        service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute);
        service.create_eye_session(
            "eye",
            basic_params_input(),
            eye_params_input(),
            Results::Relative,
        );
        service.create_object_session(
            "object",
            basic_params_input(),
            object_detection_params_input(),
            Results::Absolute,
        );

        assert!(matches!(
            service.update_basic_params(
                "ordinary",
                BasicParamsInput {
                    do_grayscale: true,
                    gradient_threshold: 7,
                },
            ),
            UpdateBasicParamsResult::Success
        ));
        assert!(service.get_basic_params("ordinary").unwrap().do_grayscale());
        assert_eq!(
            service
                .get_basic_params("ordinary")
                .unwrap()
                .gradient_threshold(),
            7
        );

        assert!(matches!(
            service.update_tile_params(
                "eye",
                TileParamsInput {
                    tile_x: 0.25,
                    tile_y: 0.75,
                },
            ),
            TileParamsUpdateResult::Success
        ));
        assert_float_eq(
            service.get_tile_params("eye").unwrap().relative_tile_x(),
            0.25,
        );
        assert_float_eq(
            service.get_tile_params("eye").unwrap().relative_tile_y(),
            0.75,
        );
        assert!(matches!(
            service.update_tile_params("ordinary", tile_params_input()),
            TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams
        ));

        assert!(matches!(
            service.update_trace_params("object", TraceParamsInput { num_skeleton: 24 },),
            TraceParamsUpdateResult::Success
        ));
        assert_eq!(
            service.get_trace_params("object").unwrap().num_skeleton(),
            24
        );
        assert!(matches!(
            service.update_trace_params("ordinary", trace_params_input()),
            TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams
        ));

        assert!(matches!(
            service.update_bucket_delta("eye", 0.6),
            BucketDeltaUpdateResult::Success
        ));
        assert_float_eq(service.get_bucket_delta("eye").unwrap(), 0.6);
        assert!(matches!(
            service.update_bucket_delta("ordinary", 0.6),
            BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta
        ));

        assert!(matches!(
            service.update_target_similarity("object", 0.82),
            TargetSimilarityUpdateResult::Success
        ));
        assert_float_eq(service.get_target_similarity("object").unwrap(), 0.82);
        assert!(matches!(
            service.update_target_similarity("ordinary", 0.82),
            TargetSimilarityUpdateResult::SessionTypeDoesNotSupportTargetSimilarity
        ));

        assert!(matches!(
            service.update_eye_params(
                "eye",
                EyeParamsInput {
                    tile_params: TileParamsInput {
                        tile_x: 0.2,
                        tile_y: 0.4,
                    },
                    bucket_delta: 0.3,
                    trace_params: TraceParamsInput { num_skeleton: 30 },
                    target_similarity: 0.88,
                },
            ),
            EyeParamsUpdateResult::Success
        ));
        assert_float_eq(service.get_bucket_delta("eye").unwrap(), 0.3);
        assert_eq!(service.get_trace_params("eye").unwrap().num_skeleton(), 30);
        assert_float_eq(service.get_target_similarity("eye").unwrap(), 0.88);
        assert!(matches!(
            service.update_eye_params("ordinary", eye_params_input()),
            EyeParamsUpdateResult::SessionTypeDoesNotSupportEyeParams
        ));

        assert!(matches!(
            service.update_object_detection_params(
                "object",
                ObjectDetectionParamsInput {
                    tile_params: TileParamsInput {
                        tile_x: 0.4,
                        tile_y: 0.4,
                    },
                    bucket_delta: 0.5,
                    trace_params: TraceParamsInput { num_skeleton: 20 },
                    target_similarity: 0.91,
                },
            ),
            ObjectDetectionParamsUpdateResult::Success
        ));
        assert_float_eq(
            service.get_tile_params("object").unwrap().relative_tile_x(),
            0.4,
        );
        assert_float_eq(service.get_bucket_delta("object").unwrap(), 0.5);
        assert_eq!(
            service.get_trace_params("object").unwrap().num_skeleton(),
            20
        );
        assert_float_eq(service.get_target_similarity("object").unwrap(), 0.91);
        assert!(matches!(
            service.update_object_detection_params("eye", object_detection_params_input()),
            ObjectDetectionParamsUpdateResult::SessionTypeDoesNotSupportObjectDetectionParams
        ));
    }

    #[test]
    fn service_adds_and_deletes_reference_objects_and_sessions() {
        let mut service = Service::new();
        service.create_object_session(
            "object",
            basic_params_input(),
            object_detection_params_input(),
            Results::Absolute,
        );
        service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute);

        assert!(matches!(
            service.add_object_to_be_detected_as_image(
                "object",
                "ref-0",
                &object_image(),
                tiny_surrounding_rectangle(),
            ),
            AddObjectToBeDetectedResult::Success
        ));

        assert!(matches!(
            service.add_object_to_be_detected_as_ascii_art("object", "ref-1", &object_ascii_art(),),
            AddObjectToBeDetectedResult::Success
        ));
        assert_eq!(
            service
                .get_object_session("object")
                .unwrap()
                .objects_to_be_detected
                .len(),
            2
        );

        assert!(matches!(
            service.add_object_to_be_detected_as_ascii_art(
                "object",
                "empty-ref",
                &empty_object_ascii_art(),
            ),
            AddObjectToBeDetectedResult::NoReferenceMosaicsDetected
        ));

        assert!(matches!(
            service.add_object_to_be_detected_as_ascii_art(
                "ordinary",
                "ref-2",
                &object_ascii_art(),
            ),
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected
        ));
        assert!(matches!(
            service.delete_reference_object("ordinary", "ref-1"),
            DeleteReferenceObjectResult::SessionTypeDoesNotSupportDeletingReferenceObject
        ));
        assert!(matches!(
            service.delete_reference_object("object", "missing"),
            DeleteReferenceObjectResult::ReferenceObjectNotFound
        ));
        assert!(matches!(
            service.delete_reference_object("object", "ref-0"),
            DeleteReferenceObjectResult::Success
        ));
        assert!(matches!(
            service.delete_reference_object("object", "ref-1"),
            DeleteReferenceObjectResult::Success
        ));
        assert_eq!(
            service
                .get_object_session("object")
                .unwrap()
                .objects_to_be_detected
                .len(),
            0
        );
        assert!(matches!(
            service.delete_session("object"),
            DeleteSessionResult::Success
        ));
        assert!(matches!(
            service.delete_session("object"),
            DeleteSessionResult::SessionNotFound
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn service_rectangles_and_enrichment_helpers_return_expected_shapes() {
        let mut service = Service::new();
        service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute);
        service.create_eye_session(
            "eye",
            basic_params_input(),
            eye_params_input(),
            Results::Relative,
        );

        assert!(matches!(
            service
                .get_rectangles("missing", solid_image([255, 255, 255]))
                .await,
            GetRectanglesResult::SessionNotFound
        ));
        let first_eye_rectangles = match service
            .get_rectangles("eye", larger_solid_image([255, 255, 255]))
            .await
        {
            GetRectanglesResult::Success(mosaics) => mosaics,
            other => panic!(
                "expected eye rectangles success, got unexpected result: {:?}",
                std::mem::discriminant(&other)
            ),
        };
        assert!(!first_eye_rectangles.is_empty());
        assert!(
            first_eye_rectangles
                .iter()
                .all(|mosaic| mosaic.color == Color::Blue)
        );

        let mosaics = calculate_ordinary_mosaics(
            BasicParams::new(false, 15),
            Results::Absolute,
            &larger_solid_image([255, 255, 255]),
        );
        assert!(!mosaics.is_empty());

        let relative_mosaics = calculate_ordinary_mosaics(
            BasicParams::new(false, 15),
            Results::Relative,
            &larger_solid_image([255, 255, 255]),
        );
        assert!(!relative_mosaics.is_empty());

        let enriched_absolute =
            deduce_enriched_mosaic(&mosaics[0], Color::Green, None, Results::Absolute);
        let enriched_relative =
            deduce_enriched_mosaic(&relative_mosaics[0], Color::Blue, None, Results::Relative);

        assert!(matches!(
            service
                .get_rectangles("ordinary", larger_solid_image([255, 255, 255]),)
                .await,
            GetRectanglesResult::Success(_)
        ));
        assert!(enriched_absolute.color == Color::Green);
        assert!(enriched_absolute.label.is_none());
        assert!(enriched_relative.color == Color::Blue);
        assert!(enriched_relative.label.is_none());
        assert!(enriched_absolute.area > 0.0);
        assert!(enriched_relative.area > 0.0);
        assert!(!enriched_absolute.slice_matrix.is_empty());
        assert_eq!(enriched_absolute.average_color.red, 255);
        assert_eq!(enriched_absolute.average_color.green, 255);
        assert_eq!(enriched_absolute.average_color.blue, 255);
        assert!(enriched_relative.center_of_mass.x >= 0.0);
        assert!(enriched_relative.center_of_mass.x <= 1.0);
        assert!(enriched_relative.center_of_mass.y >= 0.0);
        assert!(enriched_relative.center_of_mass.y <= 1.0);
    }
}
