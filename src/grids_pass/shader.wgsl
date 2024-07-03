struct Uniforms {
    world_to_clip: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vert_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.world_to_clip * vec4(position, 1.0);
}

@fragment
fn frag_main() -> @location(0) vec4<f32> {
    return vec4(0.2, 0.2, 0.2, 1.0);
}
