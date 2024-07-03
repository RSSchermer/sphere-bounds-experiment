use empa::buffer::{Buffer, Uniform};
use empa::command::{
    Draw, DrawCommandEncoder, RenderBundleEncoderDescriptor, RenderStateEncoder,
    ResourceBindingCommandEncoder,
};
use empa::device::Device;
use empa::render_pipeline::{
    ColorOutput, ColorWrite, DepthStencilTest, FragmentStageBuilder, Index16, PrimitiveAssembly,
    RenderPipelineDescriptorBuilder, VertexStageBuilder,
};
use empa::shader_module::{shader_source, ShaderSource};
use empa::texture::format::{depth24plus, rgba8unorm};
use empa::type_flag::{O, X};
use empa::{abi, buffer, CompareFunction};

use crate::renderer::MainPassBundle;

const SHADER: ShaderSource = shader_source!("shader.wgsl");

#[derive(empa::render_pipeline::Vertex, Clone, Copy, Debug)]
struct Vertex {
    #[vertex_attribute(location = 0, format = "float32x2")]
    position: [f32; 2],
}

#[derive(empa::abi::Sized, Clone, Copy, Debug)]
struct Uniforms {
    clip_to_camera: abi::Mat4x4,
    gradient_bottom: abi::Vec3<f32>,
    gradient_top: abi::Vec3<f32>,
}

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "VERTEX | FRAGMENT")]
    uniform_buffer: Uniform<'a, Uniforms>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct SkyGradientPass {
    device: Device,
    uniforms: Buffer<Uniforms, buffer::Usages<O, O, O, X, O, O, X, O, O, O>>,
    gradient_bottom: abi::Vec3<f32>,
    gradient_top: abi::Vec3<f32>,
    render_bundle: MainPassBundle,
}

impl SkyGradientPass {
    pub async fn init(
        device: Device,
        gradient_bottom: abi::Vec3<f32>,
        gradient_top: abi::Vec3<f32>,
    ) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_render_pipeline(
                &RenderPipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .primitive_assembly(PrimitiveAssembly::triangle_strip::<Index16>())
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

        let vertex_data = [
            Vertex {
                position: [1.0, -1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
        ];

        let vertices: Buffer<[Vertex], _> =
            device.create_buffer(vertex_data, buffer::Usages::vertex());

        let uniforms = device.create_buffer(
            Uniforms {
                clip_to_camera: abi::Mat4x4::default(),
                gradient_bottom,
                gradient_top,
            },
            buffer::Usages::uniform_binding().and_copy_dst(),
        );

        let bind_group = device.create_bind_group(
            &bind_group_layout,
            Resources {
                uniform_buffer: uniforms.uniform(),
            },
        );

        let render_bundle_encoder = device.create_render_bundle_encoder(
            &RenderBundleEncoderDescriptor::new::<rgba8unorm>()
                .depth_stencil_format::<depth24plus>(),
        );
        let render_bundle = render_bundle_encoder
            .set_pipeline(&pipeline)
            .set_vertex_buffers(&vertices)
            .set_bind_groups(&bind_group)
            .draw(Draw {
                vertex_count: 4,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            })
            .finish();

        SkyGradientPass {
            device,
            uniforms,
            gradient_bottom,
            gradient_top,
            render_bundle,
        }
    }

    pub fn render_bundle(&self, clip_to_camera: abi::Mat4x4) -> &MainPassBundle {
        let queue = self.device.queue();

        queue.write_buffer(
            self.uniforms.view(),
            &Uniforms {
                clip_to_camera,
                gradient_bottom: self.gradient_bottom,
                gradient_top: self.gradient_top,
            },
        );

        &self.render_bundle
    }
}
