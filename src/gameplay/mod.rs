use crate::core::input::ser::{VirtualButton, VirtualKey};
use crate::core::input::InputAction;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod delete;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Shoot,
    RotateLeft,
    RotateRight,
    Pickup,
    Boost,
}

pub fn get_default_button_mapping() -> HashMap<VirtualKey, Action> {
    let mut m = HashMap::new();
    m.insert(VirtualKey::Up, Action::MoveUp);
    m.insert(VirtualKey::W, Action::MoveUp);
    m.insert(VirtualKey::Left, Action::MoveLeft);
    m.insert(VirtualKey::A, Action::MoveLeft);
    m.insert(VirtualKey::Right, Action::MoveRight);
    m.insert(VirtualKey::D, Action::MoveRight);
    m.insert(VirtualKey::Space, Action::Boost);
    m.insert(VirtualKey::Q, Action::RotateLeft);
    m.insert(VirtualKey::E, Action::RotateRight);
    m.insert(VirtualKey::F, Action::Pickup);
    m
}

pub fn get_default_mouse_mapping() -> HashMap<VirtualButton, Action> {
    let mut m = HashMap::new();
    m.insert(VirtualButton::Button1, Action::Shoot);
    m
}
impl InputAction for Action {
    fn get_default_key_mapping() -> HashMap<VirtualKey, Self> {
        get_default_button_mapping()
    }

    fn get_default_mouse_mapping() -> HashMap<VirtualButton, Self> {
        get_default_mouse_mapping()
    }
}
