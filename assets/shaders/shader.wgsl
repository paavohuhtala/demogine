#import shared::camera::CameraUniform
#import shared::drawable::Drawable
#import shared::material_info::MaterialInfo

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> drawables: array<Drawable>;

@group(2) @binding(0)
var<storage, read> material_info: array<MaterialInfo>;
@group(2) @binding(1)
var textures: binding_array<texture_2d<f32>>;
@group(2) @binding(2)
var default_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) instance_index: u32,
}

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let drawable = drawables[instance_index];
    let world_position = drawable.model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;

    let normal_matrix = mat3x3<f32>(
        drawable.model_matrix[0].xyz,
        drawable.model_matrix[1].xyz,
        drawable.model_matrix[2].xyz
    );
    out.normal = normalize(normal_matrix * model.normal);
    out.uv = model.uv;
    out.instance_index = instance_index;
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let material_id = drawables[in.instance_index].material_id;
    let material = material_info[material_id];
    let texture_index = material.base_color;
    let texture_sample = textureSample(textures[texture_index], default_sampler, in.uv);
    let ao_sample = textureSample(textures[material.ao_roughness_metallic], default_sampler, in.uv).r;
    const ambient_light = vec3<f32>(0.1, 0.1, 0.1);

    let light_direction = normalize(vec3(0.4, 1.0, 0.1));
    let normal = normalize(in.normal);
    let intensity = dot(normal, light_direction) * ao_sample;

    let diffuse_color = texture_sample.rgb * max(intensity, 0.0);
    return vec4<f32>(diffuse_color, 1.0);
}