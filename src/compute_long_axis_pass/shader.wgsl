#include "src/sphere.wgsl"
#include "src/line.wgsl"

struct Uniforms {
    world_to_camera: mat4x4<f32>,
    camera_to_clip: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> spheres: array<Sphere>;

@group(0) @binding(2)
var<storage, read_write> long_axes: array<Line>;

// Computes the long axes of a given sphere projected in clip-space.
//
// Implementation based on an article by Inigo Quilez [0] and the accompanying shadertoy [1].
//
// [0]: https://iquilezles.org/articles/sphereproj/
// [1]: https://www.shadertoy.com/view/XdBGzd
fn long_axis_compute(
    // The coordinates of the sphere center in camera space.
    sphere_center_camera: vec3<f32>,
    // The sphere radius in camera space.
    sphere_radius_camera: f32,
    // The camera-space-to-clip-space projection matrix.
    camera_to_clip: mat4x4<f32>,
) -> Line {
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

    let scaled_center = vec2(
        -camera_to_clip[0][0] * center.x,
        -camera_to_clip[1][1] * center.y
    );

    let v = sqrt(num / div) * scaled_center;

    let projected_center = sphere_center_camera.z * scaled_center / (z_squared - r_squared);

    var long_axis = Line();

    long_axis.start = projected_center - v;
    long_axis.end = projected_center + v;

    return long_axis;
}

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    if index >= arrayLength(&spheres) {
        return;
    }

    let sphere = spheres[index];
    let sphere_center_camera = (uniforms.world_to_camera * vec4(sphere.position, 1.0)).xyz;

    long_axes[index] = long_axis_compute(
        sphere_center_camera,
        sphere.radius,
        uniforms.camera_to_clip,
    );
}
