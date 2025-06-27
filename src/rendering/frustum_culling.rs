use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use wgpu::wgt::DrawIndexedIndirectArgs;

use crate::{
    math::frustum::Frustum,
    rendering::shader_loader::{
        ComputePipelineCache, ComputePipelineId, PipelineCacheBuilder, ShaderDefinition,
    },
};

const SHADER_DEF: ShaderDefinition = ShaderDefinition {
    name: "Frustum culling compute shader",
    path: "frustum_culling.wgsl",
};

pub struct FrustumCullingResources {
    pub draw_commands_buffer: wgpu::Buffer,
    pub frustum_buffer: wgpu::Buffer,
    pub culling_bind_group: wgpu::BindGroup,
    pipeline_id: ComputePipelineId,
}

impl FrustumCullingResources {
    pub fn new(
        device: &wgpu::Device,
        instance_buffer: &wgpu::Buffer,
        mesh_info_buffer: &wgpu::Buffer,
        pipeline_builder: &mut PipelineCacheBuilder<wgpu::ComputePipeline>,
    ) -> Self {
        let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frustum buffer"),
            size: std::mem::size_of::<GpuFrustum>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draw_commands_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw commands buffer"),
            size: (crate::rendering::instancing::MAX_DRAWABLES as u32
                * std::mem::size_of::<DrawIndexedIndirectArgs>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Frustum culling bind group layout"),
            entries: &[
                // Frustum uniform buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Mesh info buffer
                {
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                },
                // Drawable buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Draw commands buffer (read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let culling_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frustum culling bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mesh_info_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: draw_commands_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_id = pipeline_builder.add_shader(
            SHADER_DEF,
            Box::new(
                move |device: &wgpu::Device, shader_def: &ShaderDefinition, source: &str| {
                    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(shader_def.name),
                        source: wgpu::ShaderSource::Wgsl(source.into()),
                    });

                    let compute_pipeline =
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Frustum culling compute pipeline"),
                            layout: Some(&device.create_pipeline_layout(
                                &wgpu::PipelineLayoutDescriptor {
                                    label: Some("Frustum culling pipeline layout"),
                                    bind_group_layouts: &[&bind_group_layout],
                                    push_constant_ranges: &[],
                                },
                            )),
                            module: &shader,
                            entry_point: Some("cull"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            cache: None,
                        });

                    Ok(compute_pipeline)
                },
            ),
        );

        Self {
            draw_commands_buffer,
            frustum_buffer,
            culling_bind_group,
            pipeline_id,
        }
    }

    pub fn update_frustum(&self, queue: &wgpu::Queue, frustum: &Frustum) {
        let gpu_frustum = GpuFrustum::from(frustum);
        queue.write_buffer(
            &self.frustum_buffer,
            0,
            bytemuck::cast_slice(&[gpu_frustum]),
        );
    }

    pub fn dispatch_culling_for_buffer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &ComputePipelineCache,
        instance_count: u32,
    ) {
        let pipeline = pipeline_cache.get(self.pipeline_id);

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Frustum culling compute pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(pipeline);
        compute_pass.set_bind_group(0, &self.culling_bind_group, &[]);

        const WORKGROUP_SIZE: u32 = 64;
        let workgroup_count = instance_count.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }
}

/// GPU representation of frustum planes (must match WGSL struct)
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct GpuFrustum {
    pub planes: [Vec4; 6],
}

impl From<&Frustum> for GpuFrustum {
    fn from(frustum: &Frustum) -> Self {
        let planes = frustum.planes.map(|plane| {
            Vec4::new(
                plane.normal.x,
                plane.normal.y,
                plane.normal.z,
                plane.distance,
            )
        });
        Self { planes }
    }
}
