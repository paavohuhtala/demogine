
struct GlobalUniforms {
    resolution: vec2<f32>,
    now: f32,
}

@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = globals.now * 0.5;

    let aspect_ratio = globals.resolution.x / globals.resolution.y;

    // UV coordinates are in the range [0, 1]
    // convert them so that [0, 0] is the center of the screen and adjust ratio is taken into account
    // so that the circle is not stretched
    let x = (in.uv.x - 0.5) * aspect_ratio * 2.0;
    let y = (in.uv.y - 0.5) * 2.0;

    var circle_center_x = 0.0;
    var circle_center_y = 0.0;

    circle_center_x += sin(time * 0.8) * 0.5;
    circle_center_y += cos(time * 0.8) * 0.5;

    // distance to circle in the center
    let radius = 0.4;
    let dist = length(vec2<f32>(x + circle_center_x, y + circle_center_y));

    let brightness = 1.0 - smoothstep(radius, radius + 0.02, fract(sin(time * 2.0 + dist)) * 0.5);

    return vec4<f32>(brightness, 0.0, 0.0, 1.0);
}