#![feature(async_closure, future_join)]

pub mod bounding_rects_pass;
pub mod camera;
pub mod camera_controller;
pub mod compute_bounds_pass;
pub mod compute_long_axis_pass;
pub mod grid;
pub mod grids_pass;
pub mod line;
pub mod long_axes_pass;
pub mod mouse_movement_tracker;
pub mod optics;
pub mod renderer;
pub mod sky_gradient_pass;
pub mod sphere;
pub mod sphere_bounds;
pub mod sphere_data;
pub mod spheres_pass;

use std::error::Error;
use std::f32::consts::PI;
use std::rc::Rc;
use arwa::console;

use arwa::dom::{selector, ParentNode};
use arwa::html::{HtmlButtonElement, HtmlCanvasElement};
use arwa::ui::UiEventTarget;
use arwa::window::window;
use empa::adapter::Feature;
use empa::arwa::{NavigatorExt, PowerPreference, RequestAdapterOptions};
use empa::{abi, buffer};
use empa::buffer::{Buffer, BufferUsages};
use empa::device::DeviceDescriptor;
use empa_glam::ToAbi;
use futures::{FutureExt, StreamExt};
use glam::{Quat, Vec3};

use crate::camera::{Camera, CameraDescriptor};
use crate::camera_controller::CameraController;
use crate::compute_bounds_pass::{ComputeSphereBounds, ComputeSphereBoundsInput};
use crate::compute_long_axis_pass::{ComputeLongAxesPass, ComputeLongAxesPassInput};
use crate::grid::{Grid, GridDescriptor};
use crate::line::Line;
use crate::optics::{Lens, PerspectiveLens};
use crate::renderer::{Renderer, RendererConfig};
use crate::sphere::Sphere;
use crate::sphere_bounds::SphereBounds;
use crate::sphere_data::SphereData;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    arwa::spawn_local(render().map(|res| res.unwrap()));
}

async fn render() -> Result<(), Box<dyn Error>> {
    let window = window();
    let document = window.document();
    let empa = window.navigator().empa();
    let canvas: HtmlCanvasElement = document
        .query_selector(&selector!("#viewer"))
        .ok_or("canvas not found")?
        .try_into()?;

    let adapter = empa
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("adapter not found")?;
    let device = adapter
        .request_device(&DeviceDescriptor {
            required_features: Feature::TimestampQuery,
            ..Default::default()
        })
        .await?;

    let mut camera = Camera::from(CameraDescriptor {
        lens: PerspectiveLens {
            fov_vertical: 0.45 * PI,
            aspect_ratio: canvas.width() as f32 / canvas.height() as f32,
            frustum_near: 0.01,
            frustum_far: 100.0,
        },
        position: Vec3::new(0.0, 0.0, 5.0),
        orientation: Quat::IDENTITY,
    });
    let camera_controller = CameraController::init(&camera, &canvas);

    let mut renderer = Renderer::init(
        device.clone(),
        canvas,
        RendererConfig {
            grids: &[Grid::from(GridDescriptor {
                scale: 1.0,
                width: 20,
                height: 20,
                position: Vec3::new(0.0, 0.0, 0.0),
                orientation: Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.5 * PI),
            })],
            gradient_bottom: Vec3::new(0.2, 0.2, 0.22),
            gradient_top: Vec3::new(0.9, 0.9, 0.95),
        },
    )
    .await;
    let compute_sphere_bounds = ComputeSphereBounds::init(device.clone()).await;
    let compute_long_axes = ComputeLongAxesPass::init(device.clone()).await;

    let sphere_data = SphereData::new(&device, 20);
    let spheres: Buffer<[Sphere], _> = device.create_buffer(
        [Sphere {
            origin: abi::Vec3(0.0, 0.0, 0.0),
            radius: 1.0,
        }],
        buffer::Usages::storage_binding(),
    );

    let sphere_bounds: Buffer<[SphereBounds], _> = device.create_slice_buffer_zeroed(spheres.len(), buffer::Usages::storage_binding().and_copy_src());
    let sphere_bounds = Rc::new(sphere_bounds);

    let sphere_bounds_readback = device.create_slice_buffer_zeroed(spheres.len(), buffer::Usages::map_read().and_copy_dst());

    let long_axes: Buffer<[Line], _> = device.create_slice_buffer_zeroed(spheres.len(), buffer::Usages::storage_binding().and_copy_src());
    let long_axes = Rc::new(long_axes);

    let long_axes_readback = device.create_slice_buffer_zeroed(spheres.len(), buffer::Usages::map_read().and_copy_dst());

    let log_bounds_button: HtmlButtonElement = document.query_selector(&selector!("#log_bounds_button")).ok_or("log bounds button not found")?.try_into()?;
    let mut on_log_bounds = log_bounds_button.on_click();

    arwa::spawn_local({
        let device = device.clone();
        let sphere_bounds = sphere_bounds.clone();

        async move {
            while let Some(_) = on_log_bounds.next().await {
                let mut encoder = device.create_command_encoder();

                encoder = encoder.copy_buffer_to_buffer_slice(sphere_bounds.view(), sphere_bounds_readback.view());

                device.queue().submit(encoder.finish());

                sphere_bounds_readback.map_read().await.unwrap();

                console::log!(format!("{:#?}", &*sphere_bounds_readback.mapped()));

                sphere_bounds_readback.unmap();
            }
        }
    });

    let log_long_axes_button: HtmlButtonElement = document.query_selector(&selector!("#log_long_axes_button")).ok_or("log axes button not found")?.try_into()?;
    let mut on_log_long_axes = log_long_axes_button.on_click();

    arwa::spawn_local({
        let device = device.clone();
        let long_axes = long_axes.clone();

        async move {
            while let Some(_) = on_log_long_axes.next().await {
                let mut encoder = device.create_command_encoder();

                encoder = encoder.copy_buffer_to_buffer_slice(long_axes.view(), long_axes_readback.view());

                device.queue().submit(encoder.finish());

                long_axes_readback.map_read().await.unwrap();

                console::log!(format!("{:#?}", &*long_axes_readback.mapped()));

                long_axes_readback.unmap();
            }
        }
    });

    loop {
        window.request_animation_frame().await;

        camera_controller.update_camera(&mut camera);

        let mut encoder = device.create_command_encoder();

        encoder = compute_sphere_bounds.encode(encoder, ComputeSphereBoundsInput {
            world_to_camera: camera.world_to_camera().to_abi(),
            camera_to_clip: camera.lens().camera_to_clip().to_abi(),
            spheres: spheres.view(),
            sphere_bounds: sphere_bounds.view(),
        });
        encoder = compute_long_axes.encode(encoder, ComputeLongAxesPassInput {
            world_to_camera: camera.world_to_camera().to_abi(),
            camera_to_clip: camera.lens().camera_to_clip().to_abi(),
            spheres: spheres.view(),
            long_axes: long_axes.view(),
        });

        device.queue().submit(encoder.finish());

        renderer.render(&sphere_data, spheres.view(), sphere_bounds.view(), long_axes.view(), &camera).await;
    }
}
