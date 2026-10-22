# Video Sentinel

**Tags:** `rust` `computer-vision` `image-processing` `graphql` `ros2` `video`

## Introduction

Video Sentinel finds image regions with similar shapes and can track them across
frames or detect configured reference objects. The Rust workspace contains the
image-processing and detection pipeline, a reusable service layer, a GraphQL
service, a ROS 2 node, and a command-line video processor.

The workspace uses Rust 2024. Build the workspace with:

```sh
cargo build --workspace
```

The applications are:

- `video-sentinel-service`: HTTPS GraphQL API, bound to `127.0.0.1` by default.
- `video-sentinel-ros2-node`: subscribes to compressed ROS 2 images and
  publishes `vision_msgs/Detection2DArray`.
- `video-sentinel-exe`: processes a video file and writes an annotated video.

## Usage: GraphQL

Start the API from the workspace root:

```sh
cargo run -p video-sentinel-service -- --port 8080
```

The GraphQL endpoint is `https://localhost:8080/graphql`; the server generates
a self-signed localhost certificate at startup. Use `curl -k` only for local
development, or configure a trusted certificate before exposing the service.
The browser GraphQL Playground is available at the same URL.

Create a session by sending one of these GraphQL mutations. Each session ID must
be unique. `results` can be `ABSOLUTE` or `RELATIVE`.

**Ordinary session** — segments each frame into mosaics without temporal
tracking:

```graphql
mutation {
  createOrdinarySession(
    sessionId: "ordinary-1"
    basicParams: { doGrayscale: true, gradientThreshold: 15 }
    results: ABSOLUTE
  )
}
```

**Eye session** — tracks matching mosaics from frame to frame:

```graphql
mutation {
  createEyeSession(
    sessionId: "eye-1"
    basicParams: { doGrayscale: true, gradientThreshold: 15 }
    eyeParams: {
      tileParams: { tileX: 0.5, tileY: 0.5 }
      bucketDelta: 0.25
      traceParams: { numSkeleton: 12 }
      targetSimilarity: 0.9
    }
    results: ABSOLUTE
  )
}
```

**Object session** — finds configured reference objects. Create the session,
then add one or more references using ASCII art:

```graphql
mutation {
  createObjectSession(
    sessionId: "object-1"
    basicParams: { doGrayscale: true, gradientThreshold: 15 }
    objectDetectionParams: {
      tileParams: { tileX: 0.5, tileY: 0.5 }
      bucketDelta: 0.25
      traceParams: { numSkeleton: 12 }
      targetSimilarity: 0.9
    }
    results: ABSOLUTE
  )
}
```

```graphql
mutation {
  addObjectToBeDetectedAsAsciiArt(
    sessionId: "object-1"
    objectId: "target"
    asciiArt: """
      ..##..
      .####.
      ######
      .####.
      ..##..
    """
  )
}
```

To process an image, call `getRectangles` with a JPEG file using the GraphQL
multipart upload convention:

```sh
curl -k https://localhost:8080/graphql \
  -F 'operations={"query":"mutation ($sessionId: String!, $image: Upload!) { getRectangles(sessionId: $sessionId, image: $image) { label color area boundingBox { topLeft { x y } bottomRight { x y } } } }","variables":{"sessionId":"eye-1","image":null}}' \
  -F 'map={"0":["variables.image"]}' \
  -F '0=@frame.jpg;type=image/jpeg'
```

The result includes each detected region's bounding box, color, area, and other
geometry and image statistics. Use `getOrdinarySession`, `getEyeSession`, or
`getObjectSession` to retrieve session state; the corresponding query takes a
`sessionId`. For example:

```graphql
query {
  getEyeSession(sessionId: "eye-1") {
    basicParams { doGrayscale gradientThreshold }
    eyeParams {
      tileParams { tileX tileY }
      bucketDelta
      traceParams { numSkeleton }
      targetSimilarity
    }
    results
  }
}
```

For object references from an image instead, use
`addObjectToBeDetectedAsImage(sessionId:, objectId:, image:, surroundingRectangle:)`
with a JPEG upload and a rectangle containing `topLeft` and `bottomRight`
points.

## Usage: ROS 2

The ROS 2 node loads one YAML configuration at startup. ROS 2 must be installed
and its environment sourced so the node can discover the configured topics and
message types. Three working examples are included:

- Ordinary: [ros2-session.example.yaml](crates/vs-ros2/config/ros2-session.example.yaml)
- Eye: [ros2-session-eye.example.yaml](crates/vs-ros2/config/ros2-session-eye.example.yaml)
- Object: [ros2-session-object.example.yaml](crates/vs-ros2/config/ros2-session-object.example.yaml)

Run a configuration from the workspace root:

```sh
cargo run -p video-sentinel-ros2 --bin video-sentinel-ros2-node -- \
  --config crates/vs-ros2/config/ros2-session.example.yaml
```

Each configuration selects a `sensor_msgs/CompressedImage` input topic under
`topics.image`. By default, ordinary and eye sessions publish rectangles to
`/video_sentinel/sessions/<session-id>/rectangles`; object sessions publish
labelled rectangles to
`/video_sentinel/sessions/<session-id>/labelled_rectangles`. Set `topics.output`
to override the output topic. The node publishes
`vision_msgs/Detection2DArray`. ROS 2 output currently requires
`results: absolute`.

The YAML session shapes follow these examples:

```yaml
# Ordinary
session:
  type: ordinary
  id: ordinary-camera
  basic_params:
    do_grayscale: false
    gradient_threshold: 15
```

```yaml
# Eye tracking
session:
  type: eye
  id: eye-camera
  results: absolute
  basic_params:
    do_grayscale: false
    gradient_threshold: 15
  eye_params:
    tile_params: { tile_x: 0.5, tile_y: 0.5 }
    bucket_delta: 0.25
    trace_params: { num_skeleton: 12 }
    target_similarity: 0.9
```

```yaml
# Reference-object detection
session:
  type: object
  id: object-camera
  results: absolute
  basic_params:
    do_grayscale: false
    gradient_threshold: 15
  object_detection_params:
    tile_params: { tile_x: 0.5, tile_y: 0.5 }
    bucket_delta: 0.25
    trace_params: { num_skeleton: 12 }
    target_similarity: 0.9
  reference_objects:
    - source: ascii_art
      object_id: target
      ascii_art: |
        .##.
        ####
        .##.
```

Object references can also use `source: image_path` with an `image_path` and
`surrounding_rectangle`; relative image paths are resolved from the YAML file.
The node rejects missing reference images and empty ASCII-art references.

## Methodology

The pipeline processes each frame in stages:

1. **Prepare image slices.** An RGB image is optionally converted to grayscale.
   The configured gradient threshold is used to calculate slices within the
   image.
2. **Assemble mosaics.** Connected slices are grouped into components and
   summarized as mosaics, including their geometry and average color.
3. **Describe shape.** Tracking and object detection convert mosaics into
   relative coordinates and radial traces. A trace samples the mosaic around
   its center of mass using the configured number of skeleton directions.
4. **Match regions.** Traces are organized into spatial buckets using tile and
   bucket parameters. Candidate matches are compared against the
   `targetSimilarity` threshold. Eye sessions compare consecutive frames;
   object sessions compare frame regions against configured reference mosaics
   and return matching object labels.
5. **Return enriched results.** Results include region geometry and image
   statistics in absolute image coordinates or relative coordinates, according
   to the session's `results` mode. The GraphQL API returns these values
   directly, while the ROS 2 adapter publishes detection arrays.

## Session parameters

Defaults below describe the ROS 2 YAML configuration when a field is omitted.
The GraphQL create-session mutations require their parameter inputs explicitly;
they do not apply these defaults. The video-file CLI has its own defaults, shown
where they differ.

| Parameter | ROS 2 default | Video CLI default | Applies to | What it configures |
| --- | --- | --- | --- | --- |
| `do_grayscale` | `false` | `true` | All sessions | Converts the input to grayscale before slice detection. |
| `gradient_threshold` | `15` | `15` | All sessions | Gradient cutoff used to identify image slices. |
| `results` | `absolute` | `absolute` | All sessions | Whether returned mosaic geometry uses image coordinates (`absolute`) or coordinates relative to the image (`relative`). ROS 2 publishing supports only `absolute`. |
| `tile_x` | `0.1` | `0.1` | Eye and object | Width of each spatial bucketing region as a fraction of the image width; valid values are greater than `0` and at most `1`. |
| `tile_y` | `0.1` | `0.1` | Eye and object | Height of each spatial bucketing region as a fraction of the image height; valid values are greater than `0` and at most `1`. |
| `bucket_delta` | `0.1` | `0.1` | Eye and object | Granularity for grouping mosaics by their area-to-bounding-circle-area ratio. |
| `num_skeleton` | `18` | `18` | Eye and object | Number of radial directions sampled to form a shape trace for matching. |
| `target_similarity` | `0.7` | `0.7` | Eye and object | Minimum trace similarity required to consider two regions a match. |
| `reference_objects` | `[]` | Required for object sessions | Object | Reference shapes used for detection. Provide one or more image-path or ASCII-art references; an empty list gives the session no configured targets. |

Ordinary sessions use only the common image and result parameters; eye sessions
use the tracking parameters to compare regions in consecutive frames, and
object sessions use those same tracking parameters to compare frame regions
with reference objects. The CLI values above apply to
`video-sentinel-exe`; its `--session-id` defaults to `video-session`.
