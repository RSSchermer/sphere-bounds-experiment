#include "src/circle.wgsl"

@group(0) @binding(0)
var<storage, read> occluder_circles: array<Circle>;

struct VertexIn {
    @location(0) vertex_position: vec2<f32>,
    @builtin(instance_index) instance_index: u32
}

@vertex
fn vert_main(v: VertexIn) -> @builtin(position) vec4<f32> {
    let circle = occluder_circles[v.instance_index];
    let p = circle.radius * (vec2(v.vertex_position.x, v.vertex_position.y * 1280.0 / 720.0)) + circle.origin;

    return vec4(p, 0.0, 1.0);
}

@fragment
fn frag_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 0.5);
}
