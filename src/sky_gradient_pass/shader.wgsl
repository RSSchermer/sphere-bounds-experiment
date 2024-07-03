struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) sky_box_direction: vec4<f32>
}

struct Uniforms {
    clip_to_camera: mat4x4<f32>,
    gradient_bottom: vec3<f32>,
    gradient_top: vec3<f32>
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vert_main(@location(0) position: vec2<f32>) -> VertexOut {
    var result = VertexOut();

    result.position = vec4(position, 1.0, 1.0);
    result.sky_box_direction = uniforms.clip_to_camera * vec4(position, 1.0, 1.0);

    return result;
}

@fragment
fn frag_main(@location(0) sky_box_direction: vec4<f32>) -> @location(0) vec4<f32> {
    let factor = (dot(normalize(sky_box_direction.xyz), vec3(0.0, 1.0, 0.0)) + 1.0) / 2.0;

    return vec4(mix(uniforms.gradient_bottom, uniforms.gradient_top, factor), 1.0);
}
