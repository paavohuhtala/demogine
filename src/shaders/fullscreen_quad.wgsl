
#import shared::fullscreen::VertexOutput;

@group(0) @binding(0)
var<uniform> globals: shared::globals::GlobalUniforms;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    return shared::fullscreen::vs_main(vertex_index);
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
    let pos = vec2<f32>(x, y);

    let r = calculate_channel(time, pos, 0.01 + cos(time) * 0.01, get_circle_pos(time, pos, 0.0));
    let g = calculate_channel(time, pos, 0.08 + sin(time) * 0.005, get_circle_pos(time, pos, 2.0));
    let b = calculate_channel(time, pos, 0.005 + sin(time) * 0.04, get_circle_pos(time, pos, 3.0));

    return vec4<f32>(r, g, b, 1.0);
}

fn get_circle_pos(time: f32, pos: vec2<f32>, offset: f32) -> vec2<f32> {
    var circle_center_x = 0.0;
    var circle_center_y = 0.0;

    circle_center_x += sin(time * 2.0 + offset) * 2.0;
    circle_center_y += cos(time * 2.0 + offset) * -3.0;

    circle_center_x += atan2(pos.y + cos(time) * 0.1, pos.x) * 12.0;
    circle_center_y += atan2(pos.x, pos.y + sin(time) * 0.1) * 5.0;

    return vec2<f32>(circle_center_x, circle_center_y);
}

fn calculate_channel(time: f32, pos: vec2<f32>, radius: f32, center: vec2<f32>) -> f32 {
    let dist = length(pos - center);
    let brightness = smoothstep(radius, radius + 1.0, tan(time * 2.0 + dist) * 0.2);
    return brightness;
}