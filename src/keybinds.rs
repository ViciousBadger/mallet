use bevy::input::mouse::MouseWheel;
use bevy::input::InputSystem;
use bevy::{prelude::*, utils::HashMap};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum KeyBind {
    Quit,

    // Camera
    MoveLeft,
    MoveRight,
    MoveBackwards,
    MoveForwards,
    MoveDown,
    MoveUp,
    FlyMode,

    // Selection
    SetSelAxisX,
    SetSelAxisY,
    SetSelAxisZ,
    AxisLockX,
    AxisLockY,
    AxisLockZ,
    AxisLockSelected,
    ResetSelAxisOffset,
    ToggleSnap,
    HoldSnap,
    SelNext,
    SelPrev,
}

pub struct KeyBindValue {
    pub kind: KeyBindKind,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

pub enum KeyBindKind {
    Keyboard(KeyCode),
    Mouse(MouseButton),
    ScrollUp,
    ScrollDown,
}

impl KeyBindValue {
    pub fn key(keycode: KeyCode) -> Self {
        Self {
            kind: KeyBindKind::Keyboard(keycode),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn mouse_button(button: MouseButton) -> Self {
        Self {
            kind: KeyBindKind::Mouse(button),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn scroll_up() -> Self {
        Self {
            kind: KeyBindKind::ScrollUp,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn scroll_down() -> Self {
        Self {
            kind: KeyBindKind::ScrollDown,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn with_control(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

#[derive(Resource, Deref)]
pub struct KeyBinds(HashMap<KeyBind, KeyBindValue>);

impl Default for KeyBinds {
    fn default() -> Self {
        let mut map = HashMap::<KeyBind, KeyBindValue>::new();

        map.insert(
            KeyBind::Quit,
            KeyBindValue::key(KeyCode::KeyQ).with_control(),
        );
        map.insert(KeyBind::MoveLeft, KeyBindValue::key(KeyCode::KeyA));
        map.insert(KeyBind::MoveRight, KeyBindValue::key(KeyCode::KeyD));
        map.insert(KeyBind::MoveBackwards, KeyBindValue::key(KeyCode::KeyS));
        map.insert(KeyBind::MoveForwards, KeyBindValue::key(KeyCode::KeyW));
        map.insert(KeyBind::MoveDown, KeyBindValue::key(KeyCode::KeyQ));
        map.insert(KeyBind::MoveUp, KeyBindValue::key(KeyCode::KeyE));
        map.insert(
            KeyBind::FlyMode,
            KeyBindValue::mouse_button(MouseButton::Right),
        );
        map.insert(KeyBind::SetSelAxisX, KeyBindValue::key(KeyCode::KeyX));
        map.insert(KeyBind::SetSelAxisY, KeyBindValue::key(KeyCode::KeyC));
        map.insert(KeyBind::SetSelAxisZ, KeyBindValue::key(KeyCode::KeyZ));
        map.insert(
            KeyBind::AxisLockX,
            KeyBindValue::key(KeyCode::KeyX).with_shift(),
        );
        map.insert(
            KeyBind::AxisLockY,
            KeyBindValue::key(KeyCode::KeyC).with_shift(),
        );
        map.insert(
            KeyBind::AxisLockZ,
            KeyBindValue::key(KeyCode::KeyZ).with_shift(),
        );
        map.insert(
            KeyBind::AxisLockSelected,
            KeyBindValue::mouse_button(MouseButton::Middle),
        );
        map.insert(
            KeyBind::ResetSelAxisOffset,
            KeyBindValue::key(KeyCode::Digit0),
        );
        map.insert(KeyBind::ToggleSnap, KeyBindValue::key(KeyCode::KeyT));
        map.insert(KeyBind::HoldSnap, KeyBindValue::key(KeyCode::AltLeft));
        map.insert(KeyBind::SelNext, KeyBindValue::scroll_down());
        map.insert(KeyBind::SelPrev, KeyBindValue::scroll_up());

        KeyBinds(map)
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<KeyBinds>()
        .init_resource::<ButtonInput<KeyBind>>()
        .add_systems(
            PreUpdate,
            (
                input_to_keybind.after(InputSystem),
                clear_keybind_inputs
                    .before(input_to_keybind)
                    .run_if(resource_exists_and_changed::<KeyBinds>),
            ),
        );
}

fn clear_keybind_inputs(mut bind_input: ResMut<ButtonInput<KeyBind>>) {
    bind_input.reset_all();
}

fn input_to_keybind(
    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keybinds: Res<KeyBinds>,
    mut scroll_input: EventReader<MouseWheel>,
    mut bind_input: ResMut<ButtonInput<KeyBind>>,
) {
    bind_input.clear();

    // Collect scroll events.
    let last_scroll = scroll_input.read().last();

    for (keybind, bind_val) in keybinds.iter() {
        let ctrl = kb_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = kb_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let alt = kb_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let main_just_pressed = match bind_val.kind {
            KeyBindKind::Keyboard(key_code) => kb_input.just_pressed(key_code),
            KeyBindKind::Mouse(mouse_button) => mouse_input.just_pressed(mouse_button),
            KeyBindKind::ScrollUp => last_scroll.is_some_and(|scroll| scroll.y > 0.0),
            KeyBindKind::ScrollDown => last_scroll.is_some_and(|scroll| scroll.y < 0.0),
        };
        if ctrl == bind_val.ctrl
            && shift == bind_val.shift
            && alt == bind_val.alt
            && main_just_pressed
        {
            bind_input.press(*keybind);
        }

        let main_just_released = match bind_val.kind {
            KeyBindKind::Keyboard(key_code) => kb_input.just_released(key_code),
            KeyBindKind::Mouse(mouse_button) => mouse_input.just_released(mouse_button),
            KeyBindKind::ScrollUp => last_scroll.is_none(),
            KeyBindKind::ScrollDown => last_scroll.is_none(),
        };
        if main_just_released {
            bind_input.release(*keybind);
        }
    }
}
