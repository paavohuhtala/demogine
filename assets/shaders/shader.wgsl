#import shared::camera::CameraUniform

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> instance_data: array<InstanceData>;

struct InstanceData {
    model_matrix: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>
}

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let instance = instance_data[instance_index];
    let world_position = instance.model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;

    let normal_matrix = mat3x3<f32>(
        instance.model_matrix[0].xyz,
        instance.model_matrix[1].xyz,
        instance.model_matrix[2].xyz
    );
    out.normal = normalize(normal_matrix * model.normal);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    const ambient_light = vec3<f32>(0.1, 0.1, 0.1);

    let light_direction = normalize(vec3(0.4, 1.0, 0.8));
    let normal = normalize(in.normal);
    let intensity = dot(normal, light_direction);
    return vec4<f32>(vec3(intensity), 1.0);
}