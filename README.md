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
      tileParams: { tileX: 0.1, tileY: 0.1 }
      bucketDelta: 0.05
      traceParams: { numSkeleton: 18 }
      targetSimilarity: 0.7
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
      tileParams: { tileX: 0.1, tileY: 0.1 }
      bucketDelta: 0.05
      traceParams: { numSkeleton: 18 }
      targetSimilarity: 0.7
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

**3D triangle session** — create the session, then send an RGB JPEG and a tightly
packed raw depth buffer. The API accepts `UINT16_MM` (`16UC1`, little-endian by
default; zero is invalid) or `FLOAT32_M` (`32FC1`); `depthIsBigendian` selects
the upload byte order. Supply both cameras' raw-image intrinsics/distortion and
the depth-optical-to-RGB-optical rigid transform when creating the session:

```graphql
mutation CreateTriangleSession {
  createThreeDTriangleSession(
    sessionId: "triangles-1"
    depthCamera: {
      width: 640, height: 480, fx: 580, fy: 580, cx: 320, cy: 240
      distortionModel: "plumb_bob", distortion: [0, 0, 0, 0, 0]
    }
    rgbCamera: {
      width: 1280, height: 720, fx: 900, fy: 900, cx: 640, cy: 360
      distortionModel: "plumb_bob", distortion: [0, 0, 0, 0, 0]
    }
    depthToRgb: {
      translationX: 0.05, translationY: 0, translationZ: 0
      rotationX: 0, rotationY: 0, rotationZ: 0, rotationW: 1
    }
  )
}
```

For `getThreeDTriangles`, multipart uploads are variables `$rgb` (JPEG) and
`$depth` (raw sample bytes). Its other required arguments are `depthWidth`,
`depthHeight`, and `depthEncoding`. Camera calibration and transform are stored
on the session and need not be resubmitted for each frame. Each camera object
contains `width`, `height`, `fx`, `fy`, `cx`, `cy`, `distortionModel`, and
`distortion`; the transform contains `translationX/Y/Z` and quaternion
`rotationX/Y/Z/W`. The operation currently returns an empty list: depth
registration and validation are implemented, but triangle extraction is left
as a TODO.

```graphql
mutation GetTriangles($sessionId: String!, $rgb: Upload!, $depth: Upload!) {
  getThreeDTriangles(
    sessionId: $sessionId
    rgb: $rgb
    depth: $depth
    depthWidth: 640
    depthHeight: 480
    depthEncoding: UINT16_MM
  ) { vertexA { x y z } vertexB { x y z } vertexC { x y z } }
}
```

Native OpenCV `calib3d` undistortion and projection primitives perform depth
registration with separate input/output resolutions; the contrib `registerDepth`
routine is avoided. Calculations are serialized per session through a bounded
worker. Concurrent `getRectangles` or `getThreeDTriangles` calls join the
calculation already in progress, and their uploads are discarded without
decoding. The ROS 2 node keeps
receiving images while calculations run, discarding them without decoding or
publishing detections for them. The video-file processor awaits each frame's
result before advancing, so every frame is processed in order.

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
- 3D triangle:
  [ros2-session-3d-triangle.example.yaml](crates/vs-ros2/config/ros2-session-3d-triangle.example.yaml)

Run a configuration from the workspace root:

```sh
cargo run -p video-sentinel-ros2 --bin video_sentinel_ros2_node -- \
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

After initialization, each node publishes an `OK` status once per second to
the standard `/diagnostics` topic (`diagnostic_msgs/DiagnosticArray`). The
status name is the fully qualified node name and `session_id` is included in
its values. A heartbeat indicates the node's receive loop is running, not that
image processing or downstream subscribers are healthy; consumers should treat
missing heartbeats as stale and also account for ROS 2 discovery.

The 3D triangle configuration uses `topics.image` for RGB `sensor_msgs/Image`,
plus `topics.depth`, `topics.rgb_camera_info`, and
`topics.depth_camera_info`. It also subscribes to `tf2_msgs/TFMessage` on
`topics.tf` and `topics.tf_static` (whose subscription uses transient-local
durability). RGB/depth images are paired with the closest CameraInfo messages
only when their frame IDs match; CameraInfo timestamp age is not constrained
because calibration can be static or low-rate. The configured
`session.frame_max_time_difference_ms` tolerance (milliseconds; zero requires
an exact RGB/depth timestamp match) applies only to RGB/depth image pairs.
Unmatched input queues, including CameraInfo, and TF histories are bounded by
`max_pending_messages`.

The depth-to-RGB transform is looked up from TF at the depth image timestamp,
following paths through parent/child frames and interpolating dynamic
transforms when bracketing samples exist. There is no identity fallback for
different frame IDs, and `CameraInfo.R`/`P` are not treated as inter-camera
extrinsics. Missing paths or unavailable timestamps cause the pair to be
dropped. CameraInfo `K`/`D` are used for their respective camera; supported
distortion models are `plumb_bob`, `rational_polynomial`, and `none`.
Depth is read from `16UC1` (millimetres, zero invalid) or `32FC1` (metres);
row stride and endianness are honored, and non-positive/non-finite samples
and points behind the RGB camera are rejected. Registration uses OpenCV
`calib3d`'s native `undistortPoints` and `projectPoints` primitives with
separate depth and RGB intrinsics/resolutions, followed by a nearest-depth
z-buffer at RGB resolution. It does not use OpenCV contrib `registerDepth`;
this avoids its documented input/output resolution-size assumption in the
OpenCV 4.10.0 implementation. The node publishes a
`visualization_msgs/Marker` with `TRIANGLE_LIST` type, the RGB image header, and
empty points until triangle extraction is implemented. Its marker frame and
timestamp therefore identify the RGB optical frame and source image time.
The `/tf_static` subscriber requests reliable, transient-local QoS to match
the standard static-transform publisher.

These behaviors follow the official [OpenCV RGB-D module
documentation](https://docs.opencv.org/4.13.0/d2/d3a/group__rgbd.html) and the
[OpenCV camera calibration and 3D reconstruction documentation](https://docs.opencv.org/4.13.0/d9/d0c/group__calib3d.html),
ROS 2 [`sensor_msgs/Image`](https://docs.ros.org/en/rolling/p/sensor_msgs/msg/Image.html)
and [`sensor_msgs/CameraInfo`](https://docs.ros.org/en/rolling/p/sensor_msgs/msg/CameraInfo.html)
message definitions, [`tf2_msgs/TFMessage`](https://docs.ros.org/en/rolling/p/tf2_msgs/msg/TFMessage.html),
[`visualization_msgs/Marker`](https://docs.ros.org/en/rolling/p/visualization_msgs/msg/Marker.html),
and [REP-118 depth-image conventions](https://www.ros.org/reps/rep-0118.html).
CameraInfo's `R` is a rectification matrix and `P` is a projection matrix,
not a general inter-camera transform. For the geometry and limits behind
`registerDepth`, see the [OpenCV 4.10.0
implementation](https://github.com/opencv/opencv_contrib/blob/4.10.0/modules/rgbd/src/depth_registration.cpp).
The TF subscription follows ROS 2's [`tf2_msgs/TFMessage`](https://docs.ros.org/en/rolling/p/tf2_msgs/msg/TFMessage.html)
and [`geometry_msgs/TransformStamped`](https://docs.ros.org/en/rolling/p/geometry_msgs/msg/TransformStamped.html)
definitions; `TransformStamped` identifies the parent/child frames and the
timestamp for the transform.

The YAML session shapes follow these examples:

```yaml
# Ordinary
session:
  type: ordinary
  id: ordinary-camera
  basic_params:
    do_grayscale: true
    gradient_threshold: 15
```

```yaml
# Eye tracking
session:
  type: eye
  id: eye-camera
  results: absolute
  basic_params:
    do_grayscale: true
    gradient_threshold: 15
  eye_params:
    tile_params: { tile_x: 0.1, tile_y: 0.1 }
    bucket_delta: 0.05
    trace_params: { num_skeleton: 18 }
    target_similarity: 0.7
```

```yaml
# Reference-object detection
session:
  type: object
  id: object-camera
  results: absolute
  basic_params:
    do_grayscale: true
    gradient_threshold: 15
  object_detection_params:
    tile_params: { tile_x: 0.1, tile_y: 0.1 }
    bucket_delta: 0.05
    trace_params: { num_skeleton: 18 }
    target_similarity: 0.7
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

The values below are the generic session defaults, matching the video-file
command-line program. GraphQL create-session mutations require parameter
inputs explicitly, so supply these values when you want the defaults there.
ROS 2 YAML applies these defaults when fields are omitted.

| Parameter | Default | Applies to | What it configures |
| --- | --- | --- | --- |
| `do_grayscale` | `true` | All sessions | Converts the input to grayscale before slice detection. |
| `gradient_threshold` | `15` | All sessions | Gradient cutoff used to identify image slices. |
| `results` | `absolute` | All sessions | Whether returned mosaic geometry uses image coordinates (`absolute`) or coordinates relative to the image (`relative`). ROS 2 publishing supports only `absolute`. |
| `tile_x` | `0.1` | Eye and object | Width of each spatial bucketing region as a fraction of the image width; valid values are greater than `0` and at most `1`. |
| `tile_y` | `0.1` | Eye and object | Height of each spatial bucketing region as a fraction of the image height; valid values are greater than `0` and at most `1`. |
| `bucket_delta` | `0.05` | Eye and object | Granularity for grouping mosaics by their area-to-bounding-circle-area ratio. |
| `num_skeleton` | `18` | Eye and object | Number of radial directions sampled to form a shape trace for matching. |
| `target_similarity` | `0.7` | Eye and object | Minimum trace similarity required to consider two regions a match. |
| `reference_objects` | None | Object | Reference shapes used for detection. Provide one or more image-path or ASCII-art references. The video CLI requires at least one object reference. |

Ordinary sessions use only the common image and result parameters; eye sessions
use the tracking parameters to compare regions in consecutive frames, and
object sessions use those same tracking parameters to compare frame regions
with reference objects. The video CLI's `--session-id` defaults to
`video-session`.
