use empa::buffer::{Buffer, Uniform};
use empa::command::{
    DrawIndexed, DrawIndexedCommandEncoder, RenderBundleEncoderDescriptor, RenderStateEncoder,
    ResourceBindingCommandEncoder,
};
use empa::device::Device;
use empa::render_pipeline::{
    ColorOutput, ColorWrite, DepthStencilTest, FragmentStageBuilder, PrimitiveAssembly,
    RenderPipelineDescriptorBuilder, VertexStageBuilder,
};
use empa::shader_module::{shader_source, ShaderSource};
use empa::texture::format::{depth24plus, rgba8unorm};
use empa::type_flag::{O, X};
use empa::{abi, buffer, CompareFunction};
use glam::{Mat4, Vec3, Vec4};

use crate::grid::Grid;
use crate::renderer::MainPassBundle;

const SHADER: ShaderSource = shader_source!("shader.wgsl");

#[derive(empa::render_pipeline::Vertex, Clone, Copy, Debug)]
struct Vertex {
    #[vertex_attribute(location = 0, format = "float32x3")]
    position: [f32; 3],
}

impl From<Vec4> for Vertex {
    fn from(vec: Vec4) -> Self {
        Vertex {
            position: [vec.x, vec.y, vec.z],
        }
    }
}

#[derive(empa::abi::Sized, Clone, Copy, Debug)]
struct Uniforms {
    world_to_clip: abi::Mat4x4,
}

#[derive(empa::resource_binding::Resources)]
struct Resources<'a> {
    #[resource(binding = 0, visibility = "VERTEX | FRAGMENT")]
    uniform_buffer: Uniform<'a, Uniforms>,
}

type ResourcesLayout = <Resources<'static> as empa::resource_binding::Resources>::Layout;

pub struct GridsPass {
    device: Device,
    uniforms: Buffer<Uniforms, buffer::Usages<O, O, O, X, O, O, X, O, O, O>>,
    render_bundle: MainPassBundle,
}

impl GridsPass {
    pub async fn init(device: Device, grids: &[Grid]) -> Self {
        let shader = device.create_shader_module(&SHADER);

        let bind_group_layout = device.create_bind_group_layout::<ResourcesLayout>();
        let pipeline_layout = device.create_pipeline_layout(&bind_group_layout);

        let pipeline = device
            .create_render_pipeline(
                &RenderPipelineDescriptorBuilder::begin()
                    .layout(&pipeline_layout)
                    .primitive_assembly(PrimitiveAssembly::line_list())
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

        let bind_group = device.create_bind_group(
            &bind_group_layout,
            Resources {
                uniform_buffer: uniforms.uniform(),
            },
        );

        let mut vertex_data: Vec<Vertex> = Vec::new();
        let mut index_data: Vec<u16> = Vec::new();
        let mut index_offset = 0;

        for grid in grids {
            let width = grid.width();
            let height = grid.height();

            let transform = Mat4::from_scale_rotation_translation(
                Vec3::new(grid.scale(), grid.scale(), grid.scale()),
                grid.orientation(),
                grid.position(),
            );

            let vertex_count = (width + 1) * (height + 1);
            let index_count = 4 + (width + height) * 2;

            vertex_data.reserve(vertex_count);
            index_data.reserve(index_count);

            let min_x = width as f32 / 2.0;
            let min_y = height as f32 / 2.0;

            // Add 4 corners in counter-clockwise order.
            vertex_data.push((transform * Vec4::new(-min_x, -min_y, 0.0, 1.0)).into());
            vertex_data.push((transform * Vec4::new(min_x, -min_y, 0.0, 1.0)).into());
            vertex_data.push((transform * Vec4::new(min_x, min_y, 0.0, 1.0)).into());
            vertex_data.push((transform * Vec4::new(-min_x, min_y, 0.0, 1.0)).into());

            // Add vertices for the "width" sides (from negative to positive)
            for i in 1..width {
                let x = i as f32 - min_x;

                vertex_data.push((transform * Vec4::new(x, -min_y, 0.0, 1.0)).into());
                vertex_data.push((transform * Vec4::new(x, min_y, 0.0, 1.0)).into());
            }

            // Add vertices for the "height" sides (from negative to positive)
            for i in 1..height {
                let y = i as f32 - min_y;

                vertex_data.push((transform * Vec4::new(-min_x, y, 0.0, 1.0)).into());
                vertex_data.push((transform * Vec4::new(min_x, y, 0.0, 1.0)).into());
            }

            // Add index data for lines connecting the corners
            index_data.push(index_offset + 0);
            index_data.push(index_offset + 1);

            index_data.push(index_offset + 1);
            index_data.push(index_offset + 2);

            index_data.push(index_offset + 2);
            index_data.push(index_offset + 3);

            index_data.push(index_offset + 3);
            index_data.push(index_offset + 0);

            // Add index data for lines between corresponding vertices on the "width" sides
            for i in 0..width - 1 {
                index_data.push(index_offset + (i * 2 + 4) as u16);
                index_data.push(index_offset + (i * 2 + 5) as u16);
            }

            // Add index data for lines between corresponding vertices on the "height" sides
            let offset = 4 + (width - 1) * 2;

            for i in 0..height - 1 {
                index_data.push(index_offset + (i * 2 + offset) as u16);
                index_data.push(index_offset + (i * 2 + offset + 1) as u16);
            }

            index_offset += index_count as u16;
        }

        let vertices: Buffer<[Vertex], _> =
            device.create_buffer(vertex_data, buffer::Usages::vertex());
        let indices: Buffer<[u16], _> = device.create_buffer(index_data, buffer::Usages::index());

        let render_bundle_encoder = device.create_render_bundle_encoder(
            &RenderBundleEncoderDescriptor::new::<rgba8unorm>()
                .depth_stencil_format::<depth24plus>(),
        );

        let render_bundle = render_bundle_encoder
            .set_pipeline(&pipeline)
            .set_vertex_buffers(&vertices)
            .set_index_buffer(&indices)
            .set_bind_groups(&bind_group)
            .draw_indexed(DrawIndexed {
                index_count: indices.len() as u32,
                instance_count: 1,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            })
            .finish();

        GridsPass {
            device,
            uniforms,
            render_bundle,
        }
    }

    pub fn render_bundle(&self, world_to_clip: abi::Mat4x4) -> &MainPassBundle {
        self.device
            .queue()
            .write_buffer(self.uniforms.view(), &Uniforms { world_to_clip });

        &self.render_bundle
    }
}
