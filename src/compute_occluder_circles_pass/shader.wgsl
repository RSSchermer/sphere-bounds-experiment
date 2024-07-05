#include "src/sphere.wgsl"
#include "src/circle.wgsl"

struct Uniforms {
    world_to_camera: mat4x4<f32>,
    camera_to_clip: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> spheres: array<Sphere>;

@group(0) @binding(2)
var<storage, read_write> occluder_circles: array<Circle>;

// Computes the the smallest circle that occludes a given sphere projected in clip-space.
//
// Implementation based on an article by Inigo Quilez [0] and the accompanying shadertoy [1].
//
// [0]: https://iquilezles.org/articles/sphereproj/
// [1]: https://www.shadertoy.com/view/XdBGzd
fn occluder_circle_compute(
    // The coordinates of the sphere center in camera space.
    sphere_center_camera: vec3<f32>,
    // The sphere radius in camera space.
    sphere_radius_camera: f32,
    // The camera-space-to-clip-space projection matrix.
    camera_to_clip: mat4x4<f32>,
) -> Circle {
    var center = sphere_center_camera;

    // If the sphere center is exactly center-screen, we hit a singularity case. If this happens, we nudge the center
    // a little bit.
    if center.x == 0.0 && center.y == 0.0 {
        center.x = 0.00001;
    }

    let r_squared = sphere_radius_camera * sphere_radius_camera;
    let l_squared = center.x * center.x + center.y * center.y;
    let z_squared = center.z * center.z;
    let k = r_squared - z_squared;
    let k_squared = k * k;

    let num = -r_squared * (r_squared - l_squared - z_squared);
    let div = l_squared * k_squared;

    let factor = num / div;

    // Note that this will specifically calculates the radius of the occluder circle relative to the X-axis in
    // clip-space; the occluder circle will need to be scaled in the Y-direction by the aspect ratio.
    let scaled_center_x = camera_to_clip[0][0] * center.xy;
    let scaled_center_len_squared = dot(scaled_center_x, scaled_center_x);
    let radius = sqrt(factor * scaled_center_len_squared);

    let scaled_center = vec2(
        -camera_to_clip[0][0] * center.x,
        -camera_to_clip[1][1] * center.y
    );

    let origin = sphere_center_camera.z * scaled_center / (z_squared - r_squared);

    return Circle(origin, radius);
}

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    if index >= arrayLength(&spheres) {
        return;
    }

    let sphere = spheres[index];
    let sphere_center_camera = (uniforms.world_to_camera * vec4(sphere.position, 1.0)).xyz;

    occluder_circles[index] = occluder_circle_compute(
        sphere_center_camera,
        sphere.radius,
        uniforms.camera_to_clip,
    );
}
