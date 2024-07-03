#include "src/sphere.wgsl"
#include "src/sphere_bounds.wgsl"

struct Uniforms {
    world_to_camera: mat4x4<f32>,
    camera_to_clip: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> spheres: array<Sphere>;

@group(0) @binding(2)
var<storage, read_write> sphere_bounds: array<SphereBounds>;

// This is based on the Mara & McGuire 2013 "2D Polyhedral Bounds of a Clipped, Perspective-Projected 3D Sphere" paper
// and accompanying code supplement. Everything is unrolled specifically for bounds along the X-axis or the Y-axis.
// We also fold in the projection transform, specifically because the perspective division (dividing the `x` and `y`
// coordinates by the `z` coorinate) allows us to elide a factor that is common to both the numerator and divisor, thus
// skipping a sqrt operation and a division.
fn sphere_bounds_for_axis(
    // Sphere center coordinate component  relevant for this axis (the `x` component for the X-axis, or
    // the `y` component for the Y-axis).
    center_c: f32,
    // The sphere center `z` coordinate component (camera space).
    center_z: f32,
    // Sphere radius (camera space).
    radius: f32,
    // Z coordinate of the near clipping plane in camera space.
    z_near: f32,
    // The `camera_to_clip` projection matrix scaling factor for the axis (`camera_to_clip[0][0]` for the X-axis,
    // `camera_to_clip[1][1]` for the Y-axis).
    projection_scale: f32,
    // Whether or not to account for the sphere potentially clipping the near viewing plane. If `false`, enables
    // fast-path that may not be correct if the sphere does clip the near viewing plane; if `true` a slower path is
    // taken that is always (conservatively) correct.
    clips_near: bool
) -> vec2<f32> {
    let projected_center_length_squared = center_c * center_c + center_z * center_z;
    let r_squared = radius * radius;
    let t_squared = projected_center_length_squared - r_squared;
    let t = sqrt(t_squared);

    var b_min_num = t * center_c + radius * center_z;
    var b_min_div = t * center_z - radius * center_c;

    var b_max_num = t * center_c - radius * center_z;
    var b_max_div = t * center_z + radius * center_c;

    if clips_near {
        let camera_inside_sphere = t_squared <= 0;

        let clip_distance = z_near - center_z;
        let sqrt_part = sqrt(r_squared - clip_distance * clip_distance);

        // When computing the `p_min` and `p_max` numerators and divisors above, we omitted a
        // `t / projected_center_length_squared` multiplication factor. We can do this because it is common to both
        // the numerators and divisors, so that when we do the perspective divide below, the factors would cancel out.
        // However, to determine whether or not we clip the near viewing plane, we compare the divisors against
        // `z_near`. To adjust for omitting the factor in the divisors, we instead scale `z_near` with the inverse
        // factor `projected_center_length_squared / t`.
        let z_near_scaled = z_near * projected_center_length_squared / t;

        if camera_inside_sphere || b_min_div > z_near_scaled {
            b_min_num = center_c - sqrt_part;
            b_min_div = z_near;
        }

        if camera_inside_sphere || b_max_div > z_near_scaled {
            b_max_num = center_c + sqrt_part;
            b_max_div = z_near;
        }
    }

    // We are now ready to compute the bounds. We can do this by dividing the numerators by the divisors, then
    // multiplying with the `projection_scale` factor, e.g.:
    //
    // b_min = projection_scale * p_min_num / p_min_div
    // b_max = projection_scale * p_max_num / p_max_div
    //
    // However, we can save a division by first computing a common divisor. We then compensate by multiplying the final
    // values with the opposite divisor at the end.

    let scale = projection_scale / (b_min_div * b_max_div);

    let b_min = b_min_num * scale * b_max_div;
    let b_max = b_max_num * scale * b_min_div;

    return vec2(b_min, b_max);
}

// Computes an axis-aligned bounding rectangle for the given sphere in clip-space.
fn sphere_bounds_compute(
    // The coordinates of the sphere center in camera space.
    sphere_center_camera: vec3<f32>,
    // The sphere radius in camera space.
    sphere_radius_camera: f32,
    // The camera-space-to-clip-space projection matrix.
    camera_to_clip: mat4x4<f32>,
    // The `z` coordinate of the near clip plane in camera space. Because the camera looks along the Z-axis in the
    // negative direction, this is typically `-near_clip_distance`.
    z_near: f32,
    // Whether or not to account for the sphere potentially clipping the near viewing plane. If `false`, enables
    // fast-path that may not be correct if the sphere does clip the near viewing plane; if `true` a slower path is
    // taken that is always (conservatively) correct.
    clips_near: bool
) -> SphereBounds {
    let x_bounds = sphere_bounds_for_axis(
        sphere_center_camera.x,
        sphere_center_camera.z,
        sphere_radius_camera,
        z_near,
        -camera_to_clip[0][0],
        clips_near,
    );
    let y_bounds = sphere_bounds_for_axis(
        sphere_center_camera.y,
        sphere_center_camera.z,
        sphere_radius_camera,
        z_near,
        -camera_to_clip[1][1],
        clips_near,
    );

    var bounds = SphereBounds();

    bounds.min.x = x_bounds[0];
    bounds.max.x = x_bounds[1];
    bounds.min.y = y_bounds[0];
    bounds.max.y = y_bounds[1];

    return bounds;
}

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    if index >= arrayLength(&spheres) {
        return;
    }

    let sphere = spheres[index];
    let sphere_center_camera = (uniforms.world_to_camera * vec4(sphere.position, 1.0)).xyz;
    let clips_near = (sphere_center_camera.z + sphere.radius) > -0.01;

    sphere_bounds[index] = sphere_bounds_compute(
        sphere_center_camera,
        sphere.radius,
        uniforms.camera_to_clip,
        -0.01,
        clips_near
    );
}
