use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use wgpu::wgt::DrawIndexedIndirectArgs;

use crate::{
    math::frustum::Frustum,
    rendering::{
        instancing::{self},
        passes::render_pass_context::ComputePassCreationContext,
        shader_loader::{ComputePipelineCache, ComputePipelineId, ShaderDefinition},
        util::bind_group_builder::BindGroupBuilder,
    },
};

const FRUSTUM_CULLING_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Frustum culling compute shader",
    path: "frustum_culling.wgsl",
};

const GENERATE_DRAWS_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Generate draw commands compute shader",
    path: "generate_draws.wgsl",
};

const GATHER_INSTANCE_DATA_SHADER: ShaderDefinition = ShaderDefinition {
    name: "Gather instance data compute shader",
    path: "gather_instance_data.wgsl",
};

pub struct DrawCommandGenerator {
    culling_pipeline_id: ComputePipelineId,
    frustum_buffer: wgpu::Buffer,
    culling_bind_group: wgpu::BindGroup,
    drawable_visibility_buffer: wgpu::Buffer,
    visible_drawables_by_mesh_buffer: wgpu::Buffer,

    generate_draws_pipeline_id: ComputePipelineId,
    generate_draws_bind_group: wgpu::BindGroup,
    pub draw_commands_buffer: wgpu::Buffer,
    pub draw_commands_count_buffer: wgpu::Buffer,

    gather_instance_data_pipeline_id: ComputePipelineId,
    gather_instance_data_bind_group: wgpu::BindGroup,
    drawable_local_indices_buffer: wgpu::Buffer,
}

impl DrawCommandGenerator {
    pub fn new(context: &mut ComputePassCreationContext) -> Self {
        let device = &context.shared.device;

        let visible_drawable_buffer = context.shared.drawable_buffers.visible_drawables.buffer();
        let drawable_buffer = context.shared.drawable_buffers.all_drawables.buffer();
        let mesh_info_buffer = &context.shared.mesh_buffers.meshes;
        let pipeline_builder = &mut context.cache_builder;

        let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frustum buffer"),
            size: std::mem::size_of::<GpuFrustum>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let drawable_visibility_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Drawable visibility buffer"),
            size: (instancing::MAX_DRAWABLES as u32 * std::mem::size_of::<u32>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let visible_drawables_by_mesh_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible drawable counts by mesh buffer"),
            size: (instancing::MAX_MESHES as u32 * std::mem::size_of::<u32>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (culling_bind_group_layout, culling_bind_group) =
            BindGroupBuilder::new("Frustum culling", wgpu::ShaderStages::COMPUTE)
                .uniform(
                    0,
                    "Frustum uniform buffer",
                    frustum_buffer.as_entire_binding(),
                )
                .storage_r(1, "Mesh info buffer", mesh_info_buffer.as_entire_binding())
                .storage_r(2, "Drawable buffer", drawable_buffer.as_entire_binding())
                .storage_rw(
                    3,
                    "Drawable visibility buffer",
                    drawable_visibility_buffer.as_entire_binding(),
                )
                .storage_rw(
                    4,
                    "Visible drawables by mesh buffer",
                    visible_drawables_by_mesh_buffer.as_entire_binding(),
                )
                .build(device);

        let culling_pipeline_id = pipeline_builder.add_shader(
            FRUSTUM_CULLING_SHADER,
            Box::new(move |device, shader_module| {
                let compute_pipeline =
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Frustum culling compute pipeline"),
                        layout: Some(&device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("Frustum culling pipeline layout"),
                                bind_group_layouts: &[&culling_bind_group_layout],
                                push_constant_ranges: &[],
                            },
                        )),
                        module: &shader_module,
                        entry_point: Some("main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        cache: None,
                    });

                Ok(compute_pipeline)
            }),
        );

        let base_offsets_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Base offsets buffer"),
            size: (instancing::MAX_MESHES as u32 * std::mem::size_of::<u32>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let draw_commands_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw commands buffer"),
            size: (instancing::MAX_MESHES as u32
                * std::mem::size_of::<DrawIndexedIndirectArgs>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let draw_commands_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw commands count buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let (generate_draws_bind_group_layout, generate_draws_bind_group) =
            BindGroupBuilder::new("Generate draws", wgpu::ShaderStages::COMPUTE)
                .storage_r(0, "Mesh info buffer", mesh_info_buffer.as_entire_binding())
                .storage_r(
                    1,
                    "Visible drawables by mesh buffer",
                    visible_drawables_by_mesh_buffer.as_entire_binding(),
                )
                .storage_rw(
                    2,
                    "Base offsets buffer",
                    base_offsets_buffer.as_entire_binding(),
                )
                .storage_rw(
                    3,
                    "Draw commands buffer",
                    draw_commands_buffer.as_entire_binding(),
                )
                .storage_rw(
                    4,
                    "Draw commands count buffer",
                    draw_commands_count_buffer.as_entire_binding(),
                )
                .build(device);

        let generate_draws_pipeline_id = pipeline_builder.add_shader(
            GENERATE_DRAWS_SHADER,
            Box::new(move |device, shader_module| {
                let compute_pipeline =
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Generate draws compute pipeline"),
                        layout: Some(&device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("Generate draws pipeline layout"),
                                bind_group_layouts: &[&generate_draws_bind_group_layout],
                                push_constant_ranges: &[],
                            },
                        )),
                        module: &shader_module,
                        entry_point: Some("main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        cache: None,
                    });

                Ok(compute_pipeline)
            }),
        );

        let drawable_local_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Drawable local indices buffer"),
            size: (instancing::MAX_MESHES as u32 * std::mem::size_of::<u32>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (gather_instance_data_bind_group_layout, gather_instance_data_bind_group) =
            BindGroupBuilder::new("Gather instance data", wgpu::ShaderStages::COMPUTE)
                .storage_r(0, "Drawable buffer", drawable_buffer.as_entire_binding())
                .storage_r(
                    1,
                    "Drawable visibility buffer",
                    drawable_visibility_buffer.as_entire_binding(),
                )
                .storage_r(
                    2,
                    "Base offsets buffer",
                    base_offsets_buffer.as_entire_binding(),
                )
                .storage_rw(
                    3,
                    "Visible drawable buffer",
                    visible_drawable_buffer.as_entire_binding(),
                )
                .storage_rw(
                    4,
                    "Drawable local indices buffer",
                    drawable_local_indices_buffer.as_entire_binding(),
                )
                .build(device);

        let gather_instance_data_pipeline_id = pipeline_builder.add_shader(
            GATHER_INSTANCE_DATA_SHADER,
            Box::new(move |device, shader_module| {
                let compute_pipeline =
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Gather instance data compute pipeline"),
                        layout: Some(&device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("Gather instance data pipeline layout"),
                                bind_group_layouts: &[&gather_instance_data_bind_group_layout],
                                push_constant_ranges: &[],
                            },
                        )),
                        module: &shader_module,
                        entry_point: Some("main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        cache: None,
                    });

                Ok(compute_pipeline)
            }),
        );

        Self {
            culling_pipeline_id,
            culling_bind_group,
            frustum_buffer,
            drawable_visibility_buffer,
            visible_drawables_by_mesh_buffer,

            generate_draws_pipeline_id,
            generate_draws_bind_group,
            draw_commands_buffer,
            draw_commands_count_buffer,

            gather_instance_data_pipeline_id,
            gather_instance_data_bind_group,
            drawable_local_indices_buffer,
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

    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline_cache: &ComputePipelineCache,
        instance_count: u32,
    ) {
        const WORKGROUP_SIZE: u32 = 64;
        let drawable_workgroup_count = instance_count.div_ceil(WORKGROUP_SIZE);

        // Reset buffers
        encoder.clear_buffer(&self.draw_commands_buffer, 0, None);
        encoder.clear_buffer(&self.draw_commands_count_buffer, 0, None);
        encoder.clear_buffer(&self.drawable_visibility_buffer, 0, None);
        encoder.clear_buffer(&self.visible_drawables_by_mesh_buffer, 0, None);
        encoder.clear_buffer(&self.drawable_local_indices_buffer, 0, None);

        {
            let pipeline = pipeline_cache.get(self.culling_pipeline_id);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Frustum culling compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &self.culling_bind_group, &[]);
            compute_pass.dispatch_workgroups(drawable_workgroup_count, 1, 1);
        }

        {
            let pipeline = pipeline_cache.get(self.generate_draws_pipeline_id);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Generate draw commands compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &self.generate_draws_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        {
            let pipeline = pipeline_cache.get(self.gather_instance_data_pipeline_id);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gather instance data compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &self.gather_instance_data_bind_group, &[]);
            compute_pass.dispatch_workgroups(drawable_workgroup_count, 1, 1);
        }
    }

    pub fn draw_commands_count_buffer(&self) -> &wgpu::Buffer {
        &self.draw_commands_count_buffer
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
