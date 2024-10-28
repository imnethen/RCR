use std::collections::HashMap;
use std::time::Instant;
use winit::event::MouseButton;
use winit::event::WindowEvent;
use winit::keyboard::PhysicalKey;

pub struct InputController {
    keys_pressed: HashMap<PhysicalKey, bool>,
    mouse_buttons_pressed: HashMap<MouseButton, bool>,
    last_frame: Instant,
    cur_frame: Instant,

    last_mouse_pos: (f32, f32),
    cur_mouse_pos: (f32, f32),
}

impl Default for InputController {
    fn default() -> Self {
        InputController {
            keys_pressed: HashMap::new(),
            mouse_buttons_pressed: HashMap::new(),

            last_frame: Instant::now(),
            cur_frame: Instant::now(),

            last_mouse_pos: (-1., -1.),
            cur_mouse_pos: (-1., -1.),
        }
    }
}

#[allow(dead_code)]
impl InputController {
    // return true if consumed event
    pub fn process_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event: ev,
                is_synthetic: _,
            } => {
                if ev.state.is_pressed() {
                    if !ev.repeat || !self.keys_pressed.contains_key(&ev.physical_key) {
                        self.keys_pressed.insert(ev.physical_key, true);
                    }
                } else {
                    self.keys_pressed.remove(&ev.physical_key);
                }

                true
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position: new_pos,
            } => {
                self.cur_mouse_pos = (new_pos.x as f32, new_pos.y as f32);
                true
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if state.is_pressed() {
                    self.mouse_buttons_pressed.insert(*button, true);
                } else {
                    self.mouse_buttons_pressed.remove(&button);
                }

                true
            }
            _ => false,
        }
    }

    // must be called at the start of every frame
    pub fn init_frame(&mut self) {
        self.last_frame = self.cur_frame;
        self.cur_frame = Instant::now();

        self.last_mouse_pos = self.cur_mouse_pos;

        self.keys_pressed.iter_mut().for_each(|(_, v)| *v = false);
        self.mouse_buttons_pressed
            .iter_mut()
            .for_each(|(_, v)| *v = false);
    }

    pub fn key_pressed(&self, key: impl Into<PhysicalKey>) -> bool {
        self.keys_pressed.contains_key(&key.into())
    }

    pub fn key_just_pressed(&self, key: impl Into<PhysicalKey>) -> bool {
        *self.keys_pressed.get(&key.into()).unwrap_or(&false)
    }

    pub fn mouse_button_pressed(&self, button: impl Into<MouseButton>) -> bool {
        self.mouse_buttons_pressed.contains_key(&button.into())
    }

    pub fn mouse_button_just_pressed(&self, button: impl Into<MouseButton>) -> bool {
        *self
            .mouse_buttons_pressed
            .get(&button.into())
            .unwrap_or(&false)
    }

    pub fn get_deltatime(&self) -> std::time::Duration {
        self.cur_frame - self.last_frame
    }

    pub fn get_mouse_delta(&self) -> (f32, f32) {
        (
            self.cur_mouse_pos.0 - self.last_mouse_pos.0,
            self.cur_mouse_pos.1 - self.last_mouse_pos.1,
        )
    }

    pub fn get_mouse_pos(&self) -> (f32, f32) {
        self.cur_mouse_pos
    }
}
