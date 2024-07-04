use std::future::join;

use arwa::html::HtmlCanvasElement;
use empa::arwa::{AlphaMode, CanvasConfiguration, ConfiguredCanvasContext, HtmlCanvasElementExt};
use empa::command::{RenderBundle, RenderPassDescriptor};
use empa::device::Device;
use empa::render_target::{
    DepthAttachment, DepthValue, FloatAttachment, LoadOp, RenderLayout, RenderTarget, StoreOp,
};
use empa::texture::format::{depth24plus, rgba8unorm};
use empa::texture::{AttachableImageDescriptor, MipmapLevels, Texture2D, Texture2DDescriptor};
use empa::type_flag::{O, X};
use empa::{buffer, texture};
use empa_glam::ToAbi;
use glam::Vec3;
use crate::bounding_rects_pass::BoundingRectsPass;

use crate::camera::Camera;
use crate::grid::Grid;
use crate::grids_pass::GridsPass;
use crate::line::Line;
use crate::long_axes_pass::LongAxesPass;
use crate::optics::Lens;
use crate::sky_gradient_pass::SkyGradientPass;
use crate::sphere::Sphere;
use crate::sphere_bounds::SphereBounds;
use crate::sphere_data::SphereData;
use crate::spheres_pass::SpheresPass;

pub struct RendererConfig<'a> {
    pub grids: &'a [Grid],
    pub gradient_bottom: Vec3,
    pub gradient_top: Vec3,
}

pub type MainPassLayout = RenderLayout<rgba8unorm, depth24plus>;
pub type MainPassBundle = RenderBundle<MainPassLayout>;

pub struct Renderer {
    device: Device,
    context: ConfiguredCanvasContext<rgba8unorm, texture::Usages<X, O, O, O, O>>,
    depth_texture: Texture2D<depth24plus, texture::Usages<X, O, O, O, O>>,
    grids_pass: GridsPass,
    sky_gradient_pass: SkyGradientPass,
    spheres_pass: SpheresPass,
    bounding_rects_pass: BoundingRectsPass,
    long_axes_pass: LongAxesPass,
}

impl Renderer {
    pub async fn init(
        device: Device,
        canvas: HtmlCanvasElement,
        config: RendererConfig<'_>,
    ) -> Self {
        let context = canvas.empa_context().configure(&CanvasConfiguration {
            device: &device,
            format: rgba8unorm,
            usage: texture::Usages::render_attachment(),
            view_formats: (),
            alpha_mode: AlphaMode::Opaque,
        });

        let depth_texture = device.create_texture_2d(&Texture2DDescriptor {
            format: depth24plus,
            usage: texture::Usages::render_attachment(),
            view_formats: (),
            width: canvas.width(),
            height: canvas.height(),
            layers: 1,
            mipmap_levels: MipmapLevels::Partial(1),
        });

        let RendererConfig {
            grids,
            gradient_bottom,
            gradient_top,
        } = config;

        let init_grids_pass = GridsPass::init(device.clone(), grids);
        let init_sky_gradient_pass = SkyGradientPass::init(
            device.clone(),
            gradient_bottom.to_abi(),
            gradient_top.to_abi(),
        );
        let init_spheres_pass = SpheresPass::init(device.clone());
        let init_bounding_rects_pass = BoundingRectsPass::init(device.clone());
        let init_long_axes_pass = LongAxesPass::init(device.clone());

        let (grids_pass, sky_gradient_pass, spheres_pass, bounding_rects_pass, long_axes_pass) =
            join!(init_grids_pass, init_sky_gradient_pass, init_spheres_pass, init_bounding_rects_pass, init_long_axes_pass).await;

        Renderer {
            device,
            context,
            depth_texture,
            grids_pass,
            sky_gradient_pass,
            spheres_pass,
            bounding_rects_pass,
            long_axes_pass,
        }
    }

    pub async fn render(
        &mut self,
        sphere_data: &SphereData,
        spheres: buffer::View<'_, [Sphere], impl buffer::StorageBinding>,
        sphere_bounds: buffer::View<'_, [SphereBounds], impl buffer::StorageBinding>,
        long_axes: buffer::View<'_, [Line], impl buffer::StorageBinding>,
        camera: &Camera<impl Lens>,
    ) {
        let world_to_clip = camera.world_to_clip().to_abi();
        let world_to_camera = camera.world_to_camera().to_abi();
        let camera_to_clip = camera.lens().camera_to_clip().to_abi();
        let clip_to_camera = camera.lens().camera_to_clip().inverse().to_abi();

        let grids_bundle = self.grids_pass.render_bundle(world_to_clip);
        let sky_bundle = self.sky_gradient_pass.render_bundle(clip_to_camera);
        let spheres_bundle = self
            .spheres_pass
            .render_bundle(world_to_clip, sphere_data, spheres);
        let bounding_rects_bundle = self.bounding_rects_pass.render_bundle(sphere_bounds);
        let long_axes_bundle = self.long_axes_pass.render_bundle(long_axes);

        let encoder = self.device.create_command_encoder();

        let mut render_pass_encoder =
            encoder.begin_render_pass(RenderPassDescriptor::new(&RenderTarget {
                color: FloatAttachment {
                    image: self
                        .context
                        .get_current_texture()
                        .attachable_image(&AttachableImageDescriptor::default()),
                    load_op: LoadOp::Clear([0.0; 4]),
                    store_op: StoreOp::Store,
                },
                depth_stencil: DepthAttachment {
                    image: self
                        .depth_texture
                        .attachable_image(&AttachableImageDescriptor::default()),
                    load_op: LoadOp::Clear(DepthValue::ONE),
                    store_op: StoreOp::Store,
                },
            }));

        render_pass_encoder = render_pass_encoder.execute_bundle(grids_bundle);
        render_pass_encoder = render_pass_encoder.execute_bundle(sky_bundle);

        if let Some(spheres_bundle) = spheres_bundle {
            render_pass_encoder = render_pass_encoder.execute_bundle(&spheres_bundle);
        }

        if let Some(bounding_rects_bundle) = bounding_rects_bundle {
            render_pass_encoder = render_pass_encoder.execute_bundle(&bounding_rects_bundle);
        }

        if let Some(long_axes_bundle) = long_axes_bundle {
            render_pass_encoder = render_pass_encoder.execute_bundle(&long_axes_bundle);
        }

        let command_buffer = render_pass_encoder.end().finish();

        self.device.queue().submit(command_buffer);
    }
}
