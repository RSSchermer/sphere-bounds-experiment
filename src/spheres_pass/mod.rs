use empa::buffer::{Buffer, Storage, Uniform};
use empa::command::{
    DrawIndexed, DrawIndexedCommandEncoder, RenderBundleEncoderDescriptor, RenderStateEncoder,
    ResourceBindingCommandEncoder,
};
use empa::device::Device;
use empa::render_pipeline::{
    ColorOutput, ColorWrite, DepthStencilTest, FragmentStageBuilder, IndexAny, PrimitiveAssembly,
    RenderPipeline, RenderPipelineDescriptorBuilder, VertexStageBuilder,
};
use empa::resource_binding::BindGroupLayout;
use empa::shader_module::{shader_source, ShaderSource};
use empa::texture::format::{depth24plus, rgba8unorm};
use empa::type_flag::{O, X};
use empa::{abi, buffer, CompareFunction};

use crate::renderer::{MainPassBundle, MainPassLayout};
use crate::sphere::Sphere;
use crate::sphere_data::{SphereData, Vertex};

const SHADER: ShaderSource = shader_source!("shader.wgsl");

#[derive(empa::abi::Sized, Clone, Copy, Debug)]
struct Uniforms {
    world_to_clip: abi::Mat4x4,
}

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "VERTEX | FRAGMENT")]
    uniform_buffer: Uniform<'a, Uniforms>,
    #[resource(binding = 1, visibility = "VERTEX | FRAGMENT")]
    spheres: Storage<'a, [Sphere]>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct SpheresPass {
    device: Device,
    bind_group_layout: BindGroupLayout<ResourcesLayout>,
    pipeline: RenderPipeline<MainPassLayout, Vertex, IndexAny, (ResourcesLayout,)>,
    uniforms: Buffer<Uniforms, buffer::Usages<O, O, O, X, O, O, X, O, O, O>>,
}

impl SpheresPass {
    pub async fn init(device: Device) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_render_pipeline(
                &RenderPipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .primitive_assembly(PrimitiveAssembly::triangle_list())
                    .vertex(
                        VertexStageBuilder::begin(&shader, "vert_main")
                            .vertex_layout::<Vertex>()
                            .finish(),
                    )
                    .fragment(
                        FragmentStageBuilder::begin(&shader, "frag_main")
                            .color_outputs(ColorOutput {
                                format: rgba8unorm,
                                write_mask: ColorWrite::All,
                            })
                            .finish(),
                    )
                    .depth_stencil_test(
                        DepthStencilTest::read_write::<depth24plus>()
                            .depth_compare(CompareFunction::LessEqual),
                    )
                    .finish(),
            )
            .await;

        let uniforms = device.create_buffer(
            Uniforms {
                world_to_clip: abi::Mat4x4::default(),
            },
            buffer::Usages::uniform_binding().and_copy_dst(),
        );

        SpheresPass {
            device,
            bind_group_layout,
            pipeline,
            uniforms,
        }
    }

    pub fn render_bundle<U>(
        &self,
        world_to_clip: abi::Mat4x4,
        sphere_data: &SphereData,
        spheres: buffer::View<[Sphere], U>,
    ) -> Option<MainPassBundle>
    where
        U: buffer::StorageBinding,
    {
        if spheres.len() == 0 {
            return None;
        }

        self.device
            .queue()
            .write_buffer(self.uniforms.view(), &Uniforms { world_to_clip });

        let bind_group = self.device.create_bind_group(
            &self.bind_group_layout,
            Resources {
                uniform_buffer: self.uniforms.uniform(),
                spheres: spheres.storage(),
            },
        );

        let render_bundle_encoder = self.device.create_render_bundle_encoder(
            &RenderBundleEncoderDescriptor::new::<rgba8unorm>()
                .depth_stencil_format::<depth24plus>(),
        );

        let bundle = render_bundle_encoder
            .set_pipeline(&self.pipeline)
            .set_vertex_buffers(&sphere_data.vertices)
            .set_index_buffer(&sphere_data.indices)
            .set_bind_groups(&bind_group)
            .draw_indexed(DrawIndexed {
                index_count: sphere_data.indices.len() as u32,
                instance_count: spheres.len() as u32,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            })
            .finish();

        Some(bundle)
    }
}
