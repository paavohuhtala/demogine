#import shared::camera::CameraUniform
#import shared::drawable::VisibleDrawable
#import shared::mesh_info::MeshInfo
#import shared::material_info::MaterialInfo

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> drawables: array<VisibleDrawable>;

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
    @location(3) tangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) instance_index: u32,
}

struct GBufferOutput {
    @location(0) color_roughness: vec4<f32>,
    @location(1) normal_metallic: vec4<f32>,
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
fn fs_main(in: VertexOutput) -> GBufferOutput {
    var out: GBufferOutput;

    let material_id = drawables[in.instance_index].material_id;
    let material = material_info[material_id];

    let base_texture_index = material.base_color;
    let base_texture_sample = textureSample(textures[base_texture_index], default_sampler, in.uv);
    let base_color = base_texture_sample.rgb;

    let normal_index = material.normal;
    let normal_texture_sample = textureSample(textures[normal_index], default_sampler, in.uv);
    let normal = normalize(normal_texture_sample.rgb * 2.0 - 1.0);

    let ao_roughness_metallic_index = material.ao_roughness_metallic;
    let ao_roughness_metallic_sample = textureSample(textures[ao_roughness_metallic_index], default_sampler, in.uv);
    let ao = ao_roughness_metallic_sample.r;
    let metallic = ao_roughness_metallic_sample.g;
    let roughness = ao_roughness_metallic_sample.b;

    out.color_roughness = vec4<f32>(base_color, roughness);
    out.normal_metallic = vec4<f32>(normal, metallic);

    return out;
}