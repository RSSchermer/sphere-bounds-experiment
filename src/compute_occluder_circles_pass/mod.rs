use bytemuck::Zeroable;
use empa::access_mode::ReadWrite;
use empa::buffer::{Storage, StorageBinding, Uniform};
use empa::command::{CommandEncoder, DispatchWorkgroups, ResourceBindingCommandEncoder};
use empa::compute_pipeline::{
    ComputePipeline, ComputePipelineDescriptorBuilder, ComputeStageBuilder,
};
use empa::device::Device;
use empa::resource_binding::BindGroupLayout;
use empa::shader_module::{shader_source, ShaderSource};
use empa::{abi, buffer};

use crate::circle::Circle;
use crate::sphere::Sphere;

const GROUP_SIZE: u32 = 256;

const SHADER: ShaderSource = shader_source!("shader.wgsl");

#[derive(abi::Sized, Clone, Copy, Debug, Zeroable)]
struct Uniforms {
    world_to_camera: abi::Mat4x4,
    camera_to_clip: abi::Mat4x4,
}

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "COMPUTE")]
    uniforms: Uniform<'a, Uniforms>,
    #[resource(binding = 1, visibility = "COMPUTE")]
    spheres: Storage<'a, [Sphere]>,
    #[resource(binding = 2, visibility = "COMPUTE")]
    occluder_circles: Storage<'a, [Circle], ReadWrite>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct ComputeOccluderCirclesPassInput<'a, U0, U1> {
    pub world_to_camera: abi::Mat4x4,
    pub camera_to_clip: abi::Mat4x4,
    pub spheres: buffer::View<'a, [Sphere], U0>,
    pub occluder_circles: buffer::View<'a, [Circle], U1>,
}

pub struct ComputeOccluderCirclesPass {
    device: Device,
    bind_group_layout: BindGroupLayout<ResourcesLayout>,
    pipeline: ComputePipeline<(ResourcesLayout,)>,
}

impl ComputeOccluderCirclesPass {
    pub async fn init(device: Device) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_compute_pipeline(
                &ComputePipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .compute(ComputeStageBuilder::begin(&shader, "main").finish())
                    .finish(),
            )
            .await;

        ComputeOccluderCirclesPass {
            device,
            bind_group_layout,
            pipeline,
        }
    }

    pub fn encode(
        &self,
        encoder: CommandEncoder,
        input: ComputeOccluderCirclesPassInput<impl StorageBinding, impl StorageBinding>,
    ) -> CommandEncoder {
        let ComputeOccluderCirclesPassInput {
            world_to_camera,
            camera_to_clip,
            spheres,
            occluder_circles,
        } = input;

        let uniforms = self.device.create_buffer(
            Uniforms {
                world_to_camera,
                camera_to_clip,
            },
            buffer::Usages::uniform_binding(),
        );

        let workgroups = (spheres.len() as u32).div_ceil(GROUP_SIZE);

        let bind_group = self.device.create_bind_group(
            &self.bind_group_layout,
            Resources {
                uniforms: uniforms.uniform(),
                spheres: spheres.storage(),
                occluder_circles: occluder_circles.storage(),
            },
        );

        encoder
            .begin_compute_pass()
            .set_pipeline(&self.pipeline)
            .set_bind_groups(&bind_group)
            .dispatch_workgroups(DispatchWorkgroups {
                count_x: workgroups,
                count_y: 1,
                count_z: 1,
            })
            .end()
    }
}
