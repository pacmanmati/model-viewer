struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
};

[[block]]
struct Locals {
    transform: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> r_locals: Locals;

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = r_locals.transform * vec4<f32>(position, 1.0);
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return vec4<f32>(normalize(in.position.xyz), 1.0);
}