#include "src/line.wgsl"

@group(0) @binding(0)
var<storage, read> long_axes: array<Line>;

@vertex
fn vert_main(
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32
) -> @builtin(position) vec4<f32> {
    let axis = long_axes[instance_index];

    var p: vec2<f32>;

    if vertex_index == 0 {
        p = axis.start;
    } else {
        p = axis.end;
    }

    return vec4(p, 0.0, 1.0);
}

@fragment
fn frag_main() -> @location(0) vec4<f32> {
    return vec4(0.0, 1.0, 0.0, 1.0);
}
