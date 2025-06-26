use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use wgpu::wgt::DrawIndexedIndirectArgs;

use crate::math::frustum::Frustum;

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

pub struct FrustumCullingResources {
    pub draw_commands_buffer: wgpu::Buffer,
    pub frustum_buffer: wgpu::Buffer,
    pub culling_bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
}

impl FrustumCullingResources {
    pub fn new(
        device: &wgpu::Device,
        instance_buffer: &wgpu::Buffer,
        primitive_buffer: &wgpu::Buffer,
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
                // Primitive buffer
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
                    resource: primitive_buffer.as_entire_binding(),
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

        let shader_source = std::fs::read_to_string("assets/shaders/frustum_culling.wgsl")
            .expect("Failed to read frustum culling shader");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Frustum culling compute shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Frustum culling compute pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Frustum culling pipeline layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &shader,
            entry_point: Some("cull"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            draw_commands_buffer,
            frustum_buffer,
            culling_bind_group,
            compute_pipeline,
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
        instance_count: u32,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Frustum culling compute pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.culling_bind_group, &[]);

        const WORKGROUP_SIZE: u32 = 64;
        let workgroup_count = instance_count.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }
}
