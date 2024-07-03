#include "src/sphere.wgsl"

struct Uniforms {
    world_to_clip: mat4x4<f32>,
}

struct VertexIn {
    @location(0) vertex_position: vec3<f32>,
    @builtin(instance_index) instance_index: u32
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(perspective) normal: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> spheres: array<Sphere>;

@vertex
fn vert_main(input: VertexIn) -> VertexOut {
    let sphere = spheres[input.instance_index];
    let vertex_model = sphere.radius * input.vertex_position + sphere.position;
    let normal = normalize(input.vertex_position);

    var result = VertexOut();

    result.position = uniforms.world_to_clip * vec4(vertex_model, 1.0);
    result.normal = (uniforms.world_to_clip * vec4(normal, 0.0)).xyz;

    return result;
}

@fragment
fn frag_main(@location(0) normal: vec3<f32>) -> @location(0) vec4<f32> {
    let camera_irradiance = max(0.0, dot(vec3(0.0, 0.0, -1.0), normal));
    let irradiance = vec3(0.2) + 0.8 * vec3(camera_irradiance);

    let color = vec3(1.0, 0.0, 0.0) * irradiance;

    return vec4(color, 1.0);
}
