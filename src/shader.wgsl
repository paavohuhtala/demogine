struct CameraUniform {
  view_proj: mat4x4<f32>
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,

  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,

  @location(0) normal: vec3<f32>
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let model_matrix = mat4x4<f32>(
        model.model_matrix_0,
        model.model_matrix_1,
        model.model_matrix_2,
        model.model_matrix_3
    );

    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.normal = model.normal;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    const light_direction = vec3<f32>(0.0, 1.0, 0.0);
    let normal = normalize(in.normal);
    let intensity = dot(normal, light_direction);
    return vec4<f32>(intensity, intensity, intensity, 1.0);
}