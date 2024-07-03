use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

use arwa::dom::Document;
use arwa::event::{Event, EventIteratorOptions};
use arwa::html::HtmlCanvasElement;
use arwa::spawn_local;
use arwa::ui::{ContextMenuEvent, PointerButton, PointerButtonEvent, UiEventTarget};
use arwa::window::window;
use futures::{future, FutureExt, StreamExt};
use glam::{Quat, Vec3};

use crate::camera::Camera;
use crate::mouse_movement_tracker::MouseMovementTracker;
use crate::optics::Lens;

pub struct CameraController {
    data: Rc<RefCell<ControllerData>>,
}

impl CameraController {
    pub fn init<L>(camera: &Camera<L>, canvas_element: &HtmlCanvasElement) -> Self
    where
        L: Lens,
    {
        let data = Rc::new(RefCell::new(ControllerData {
            camera_transform: CameraTransform {
                position: camera.position(),
                orientation: camera.orientation(),
                orbit_point: Vec3::new(0.0, 0.0, 0.0),
            },
            mouse_wheel_delta: 0.0,
            mouse_tracking_session: None,
        }));

        let data_clone = data.clone();

        spawn_local(
            canvas_element
                .on_context_menu()
                .with_options(EventIteratorOptions {
                    passive: false,
                    ..Default::default()
                })
                .for_each(async move |event: ContextMenuEvent<_>| {
                    event.prevent_default();
                }),
        );

        spawn_local(
            canvas_element
                .on_wheel()
                .with_options(EventIteratorOptions {
                    passive: false,
                    ..Default::default()
                })
                .for_each(move |event| {
                    event.prevent_default();

                    data_clone.borrow_mut().mouse_wheel_delta += event.delta_y() as f32;

                    future::ready(())
                }),
        );

        let data_clone = data.clone();
        let canvas_clone = canvas_element.clone();

        spawn_local(
            canvas_element
                .on_pointer_down()
                .with_options(EventIteratorOptions {
                    passive: false,
                    ..Default::default()
                })
                .for_each(move |event| {
                    event.prevent_default();

                    let data_clone_clone = data_clone.clone();
                    let mut data = data_clone.borrow_mut();

                    match event.button() {
                        PointerButton::Auxiliary => {
                            data.mouse_tracking_session = Some(
                                SidleSession::init(&canvas_clone, data.camera_transform).into(),
                            );

                            spawn_local(canvas_clone.on_pointer_up().into_future().map(
                                move |_| {
                                    data_clone_clone.borrow_mut().mouse_tracking_session = None;
                                },
                            ));
                        }
                        PointerButton::Secondary => {
                            data.mouse_tracking_session = Some(
                                OrbitSession::init(&canvas_clone, data.camera_transform).into(),
                            );

                            spawn_local(canvas_clone.on_pointer_up().into_future().map(
                                move |_| {
                                    data_clone_clone.borrow_mut().mouse_tracking_session = None;
                                },
                            ));
                        }
                        _ => (),
                    }

                    future::ready(())
                }),
        );

        CameraController { data }
    }

    pub fn update_camera<L>(&self, camera: &mut Camera<L>)
    where
        L: Lens,
    {
        let mut data = self.data.borrow_mut();

        if let Some(session) = &data.mouse_tracking_session {
            data.camera_transform = session.current_transform();
        } else {
            let camera_transform = data.camera_transform;
            let difference = camera_transform.orbit_point - camera_transform.position;
            let distance = difference.length();
            let direction = difference.normalize();
            let translation = direction * (data.mouse_wheel_delta / 1000.0);

            if translation.length() < distance || data.mouse_wheel_delta < 0.0 {
                data.camera_transform.position += translation;
            }

            data.mouse_wheel_delta = 0.0;
        }

        camera.set_position(data.camera_transform.position);
        camera.set_orientation(data.camera_transform.orientation);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct CameraTransform {
    position: Vec3,
    orientation: Quat,
    orbit_point: Vec3,
}

impl CameraTransform {
    fn up(&self) -> Vec3 {
        self.orientation * Vec3::new(0.0, 1.0, 0.0)
    }

    fn right(&self) -> Vec3 {
        self.orientation * Vec3::new(1.0, 0.0, 0.0)
    }
}

struct ControllerData {
    camera_transform: CameraTransform,
    mouse_wheel_delta: f32,
    mouse_tracking_session: Option<MouseTrackingSession>,
}

enum MouseTrackingSession {
    Orbit(OrbitSession),
    Sidle(SidleSession),
}

impl MouseTrackingSession {
    fn current_transform(&self) -> CameraTransform {
        match self {
            MouseTrackingSession::Orbit(session) => session.current_transform(),
            MouseTrackingSession::Sidle(session) => session.current_transform(),
        }
    }
}

impl From<OrbitSession> for MouseTrackingSession {
    fn from(session: OrbitSession) -> Self {
        MouseTrackingSession::Orbit(session)
    }
}

impl From<SidleSession> for MouseTrackingSession {
    fn from(session: SidleSession) -> Self {
        MouseTrackingSession::Sidle(session)
    }
}

struct OrbitSession {
    tracker: MouseMovementTracker,
    orbit_point: Vec3,
    initial_orientation: Quat,
    initial_relative_position: Vec3,
}

impl OrbitSession {
    fn init(canvas_element: &HtmlCanvasElement, initial_transform: CameraTransform) -> Self {
        let CameraTransform {
            position: initial_position,
            orientation: initial_orientation,
            orbit_point,
        } = initial_transform;

        OrbitSession {
            tracker: MouseMovementTracker::pointer_locked(canvas_element),
            orbit_point,
            initial_orientation,
            initial_relative_position: initial_position - orbit_point,
        }
    }

    fn current_transform(&self) -> CameraTransform {
        let mouse_movement = self.tracker.movement();

        let pan_angle = -mouse_movement.x as f32 / 400.0 * PI;
        let pan = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), pan_angle);

        let panned_orientation = pan * self.initial_orientation;
        let right = panned_orientation * Vec3::new(1.0, 0.0, 0.0);

        let tilt_angle = -mouse_movement.y as f32 / 400.0 * PI;
        let tilt = Quat::from_axis_angle(right, tilt_angle);

        let rotation = tilt * pan;
        let relative_translation = rotation * self.initial_relative_position;

        CameraTransform {
            position: self.orbit_point + relative_translation,
            orientation: rotation * self.initial_orientation,
            orbit_point: self.orbit_point,
        }
    }
}

impl Drop for OrbitSession {
    fn drop(&mut self) {
        window().document().exit_pointer_lock();
    }
}

struct SidleSession {
    tracker: MouseMovementTracker,
    orientation: Quat,
    initial_position: Vec3,
    initial_orbit_point: Vec3,
    up: Vec3,
    right: Vec3,
}

impl SidleSession {
    fn init(canvas_element: &HtmlCanvasElement, initial_transform: CameraTransform) -> Self {
        SidleSession {
            tracker: MouseMovementTracker::pointer_locked(canvas_element),
            orientation: initial_transform.orientation,
            initial_position: initial_transform.position,
            initial_orbit_point: initial_transform.orbit_point,
            up: initial_transform.up(),
            right: initial_transform.right(),
        }
    }

    fn current_transform(&self) -> CameraTransform {
        let mouse_movement = self.tracker.movement();

        let translation =
            self.up * mouse_movement.y as f32 / 80.0 + self.right * -mouse_movement.x as f32 / 80.0;

        CameraTransform {
            position: self.initial_position + translation,
            orientation: self.orientation,
            orbit_point: self.initial_orbit_point + translation,
        }
    }
}
