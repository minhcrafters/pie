use glam::Vec2;
use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use std::collections::HashSet;

pub struct InputState {
    pub pressed_keys: HashSet<String>,
    pub mouse_buttons: HashSet<MouseButton>,
    pub mouse_pos: Vec2,
    pub mouse_rel: Vec2,
    pub mouse_captured: bool,
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            pressed_keys: HashSet::new(),
            mouse_buttons: HashSet::new(),
            mouse_pos: Vec2::ZERO,
            mouse_rel: Vec2::ZERO,
            mouse_captured: false,
        }
    }

    pub fn process_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {
                keycode: Some(k), ..
            } => {
                self.pressed_keys.insert(k.name());
            }
            Event::KeyUp {
                keycode: Some(k), ..
            } => {
                self.pressed_keys.remove(&k.name());
            }
            Event::MouseButtonDown { mouse_btn, .. } => {
                self.mouse_buttons.insert(*mouse_btn);
            }
            Event::MouseButtonUp { mouse_btn, .. } => {
                self.mouse_buttons.remove(mouse_btn);
            }
            Event::MouseMotion {
                x, y, xrel, yrel, ..
            } => {
                self.mouse_pos = Vec2::new(*x as f32, *y as f32);
                self.mouse_rel += Vec2::new(*xrel as f32, *yrel as f32);
            }
            _ => {}
        }
    }

    pub fn prepare_update(&mut self) {
        self.mouse_rel = Vec2::ZERO;
    }

    pub fn is_key_down(&self, key: &str) -> bool {
        self.pressed_keys.contains(key)
    }

    pub fn is_mouse_down(&self, btn: MouseButton) -> bool {
        self.mouse_buttons.contains(&btn)
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
