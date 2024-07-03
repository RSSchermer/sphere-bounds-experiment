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

    var p_min_num = t * center_c + radius * center_z;
    var p_min_div = t * center_z - radius * center_c;

    var p_max_num = t * center_c - radius * center_z;
    var p_max_div = t * center_z + radius * center_c;

    if clips_near {
        let factor = t / projected_center_length_squared;

        p_min_num *= factor;
        p_min_div *= factor;
        p_max_num *= factor;
        p_max_div *= factor;

        let camera_inside_sphere = t_squared <= 0;

        let clip_distance = z_near - center_z;
        let sqrt_part = sqrt(r_squared - clip_distance * clip_distance);

        if camera_inside_sphere || p_min_div > z_near {
            p_min_num = center_c - sqrt_part;
            p_min_div = z_near;
        }

        if camera_inside_sphere || p_max_div > z_near {
            p_max_num = center_c + sqrt_part;
            p_max_div = z_near;
        }
    }

    let scale = projection_scale / (p_min_div * p_max_div);

    let p_min = p_min_num * scale * p_max_div;
    let p_max = p_max_num * scale * p_min_div;

    return vec2(p_min, p_max);
}

//// Ported from https://zeux.io/2023/01/12/approximate-projected-bounds/
fn sphere_bounds_compute(
    sphere_center_camera: vec3<f32>,
    sphere_radius_camera: f32,
    camera_to_clip: mat4x4<f32>,
    z_near: f32,
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