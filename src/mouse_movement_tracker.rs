use std::cell::RefCell;
use std::rc::Rc;

use arwa::dom::{Document, Element};
use arwa::html::HtmlCanvasElement;
use arwa::spawn_local;
use arwa::ui::{ModifierState, PointerPositionState, UiEventTarget};
use arwa::window::window;
use futures::future::AbortHandle;
use futures::stream::Abortable;
use futures::{future, StreamExt};
use glam::IVec2;

pub struct MouseMovementTracker {
    data: Rc<RefCell<Data>>,
    pointer_lock: bool,
    abort_handle: AbortHandle,
}

impl MouseMovementTracker {
    pub fn new(canvas_element: &HtmlCanvasElement) -> Self {
        let data = Rc::new(RefCell::new(Data {
            movement: IVec2::new(0, 0),
            origin: None,
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
        }));

        let data_clone = data.clone();

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let on_move = canvas_element.on_pointer_move();
        let abortable = Abortable::new(on_move, abort_registration);

        spawn_local(abortable.for_each(move |event| {
            let mut data = data_clone.borrow_mut();

            if let Some(origin) = data.origin {
                let offset = IVec2::new(event.offset_x() as i32, event.offset_y() as i32);

                data.movement = offset - origin;
            } else {
                let offset = IVec2::new(event.offset_x() as i32, event.offset_y() as i32);
                let movement = IVec2::new(event.movement_x() as i32, event.movement_y() as i32);

                data.origin = Some(offset - movement);
                data.movement = movement;
            }

            data.ctrl_key = event.ctrl_key();
            data.shift_key = event.shift_key();
            data.alt_key = event.alt_key();
            data.meta_key = event.meta_key();

            future::ready(())
        }));

        MouseMovementTracker {
            data,
            pointer_lock: false,
            abort_handle,
        }
    }

    pub fn pointer_locked(canvas_element: &HtmlCanvasElement) -> Self {
        let data = Rc::new(RefCell::new(Data {
            movement: IVec2::new(0, 0),
            origin: None,
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
        }));

        let data_clone = data.clone();

        canvas_element.request_pointer_lock();

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let on_move = canvas_element.on_pointer_move();
        let abortable = Abortable::new(on_move, abort_registration);

        spawn_local(abortable.for_each(move |event| {
            let mut data = data_clone.borrow_mut();

            data.movement += IVec2::new(event.movement_x() as i32, event.movement_y() as i32);
            data.ctrl_key = event.ctrl_key();
            data.shift_key = event.shift_key();
            data.alt_key = event.alt_key();
            data.meta_key = event.meta_key();

            future::ready(())
        }));

        MouseMovementTracker {
            data,
            pointer_lock: true,
            abort_handle,
        }
    }

    pub fn movement(&self) -> IVec2 {
        self.data.borrow().movement
    }

    pub fn ctrl_key(&self) -> bool {
        self.data.borrow().ctrl_key
    }

    pub fn shift_key(&self) -> bool {
        self.data.borrow().shift_key
    }

    pub fn alt_key(&self) -> bool {
        self.data.borrow().alt_key
    }

    pub fn meta_key(&self) -> bool {
        self.data.borrow().meta_key
    }
}

impl Drop for MouseMovementTracker {
    fn drop(&mut self) {
        self.abort_handle.abort();

        if self.pointer_lock {
            window().document().exit_pointer_lock();
        }
    }
}

struct Data {
    movement: IVec2,
    origin: Option<IVec2>,
    ctrl_key: bool,
    shift_key: bool,
    alt_key: bool,
    meta_key: bool,
}
