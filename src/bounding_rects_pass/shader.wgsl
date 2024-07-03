#include "src/sphere_bounds.wgsl"

@group(0) @binding(0)
var<storage, read> sphere_bounds: array<SphereBounds>;

@vertex
fn vert_main(
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32
) -> @builtin(position) vec4<f32> {
    let bounds = sphere_bounds[instance_index];

    var p: vec2<f32>;

    if vertex_index == 0 {
        p = bounds.min;
    } else if vertex_index == 1 {
        p = vec2(bounds.max.x, bounds.min.y);
    } else if vertex_index == 2 {
        p = bounds.max;
    } else {
        p = vec2(bounds.min.x, bounds.max.y);
    }

    return vec4(p, 0.0, 1.0);
}

@fragment
fn frag_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
