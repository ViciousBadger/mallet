use bevy::input::mouse::MouseWheel;
use bevy::input::InputSystem;
use bevy::{prelude::*, utils::HashMap};

/// Inputs bound to application actions.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum Binding {
    Quit,
    Playtest,

    // Camera
    MoveLeft,
    MoveRight,
    MoveBackwards,
    MoveForwards,
    MoveDown,
    MoveUp,
    LookLeft,
    LookRight,
    LookUp,
    LookDown,
    FlyMode,
    FlySpeedUp,
    FlySpeedDown,

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

pub struct BoundInput {
    pub source: BoundInputSource,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

pub enum BoundInputSource {
    Keyboard(KeyCode),
    Mouse(MouseButton),
    ScrollUp,
    ScrollDown,
}

impl BoundInput {
    pub fn key(keycode: KeyCode) -> Self {
        Self {
            source: BoundInputSource::Keyboard(keycode),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn mouse_button(button: MouseButton) -> Self {
        Self {
            source: BoundInputSource::Mouse(button),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn scroll_up() -> Self {
        Self {
            source: BoundInputSource::ScrollUp,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn scroll_down() -> Self {
        Self {
            source: BoundInputSource::ScrollDown,
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
pub struct InputBindings(HashMap<Binding, BoundInput>);

impl Default for InputBindings {
    fn default() -> Self {
        let mut map = HashMap::<Binding, BoundInput>::new();

        map.insert(Binding::Quit, BoundInput::key(KeyCode::KeyQ).with_control());
        map.insert(Binding::Playtest, BoundInput::key(KeyCode::Tab));
        map.insert(Binding::MoveLeft, BoundInput::key(KeyCode::KeyA));
        map.insert(Binding::MoveRight, BoundInput::key(KeyCode::KeyD));
        map.insert(Binding::MoveBackwards, BoundInput::key(KeyCode::KeyS));
        map.insert(Binding::MoveForwards, BoundInput::key(KeyCode::KeyW));
        map.insert(Binding::MoveDown, BoundInput::key(KeyCode::KeyQ));
        map.insert(Binding::MoveUp, BoundInput::key(KeyCode::KeyE));
        map.insert(Binding::LookLeft, BoundInput::key(KeyCode::ArrowLeft));
        map.insert(Binding::LookRight, BoundInput::key(KeyCode::ArrowRight));
        map.insert(Binding::LookDown, BoundInput::key(KeyCode::ArrowDown));
        map.insert(Binding::LookUp, BoundInput::key(KeyCode::ArrowUp));
        map.insert(
            Binding::FlyMode,
            BoundInput::mouse_button(MouseButton::Right),
        );
        map.insert(Binding::FlySpeedUp, BoundInput::scroll_up());
        map.insert(Binding::FlySpeedDown, BoundInput::scroll_down());
        map.insert(Binding::SetSelAxisX, BoundInput::key(KeyCode::KeyX));
        map.insert(Binding::SetSelAxisY, BoundInput::key(KeyCode::KeyC));
        map.insert(Binding::SetSelAxisZ, BoundInput::key(KeyCode::KeyZ));
        map.insert(
            Binding::AxisLockX,
            BoundInput::key(KeyCode::KeyX).with_shift(),
        );
        map.insert(
            Binding::AxisLockY,
            BoundInput::key(KeyCode::KeyC).with_shift(),
        );
        map.insert(
            Binding::AxisLockZ,
            BoundInput::key(KeyCode::KeyZ).with_shift(),
        );
        map.insert(
            Binding::AxisLockSelected,
            BoundInput::mouse_button(MouseButton::Middle),
        );
        map.insert(
            Binding::ResetSelAxisOffset,
            BoundInput::key(KeyCode::Digit0),
        );
        map.insert(Binding::ToggleSnap, BoundInput::key(KeyCode::KeyT));
        map.insert(Binding::HoldSnap, BoundInput::key(KeyCode::AltLeft));
        map.insert(Binding::SelNext, BoundInput::scroll_down().with_shift());
        map.insert(Binding::SelPrev, BoundInput::scroll_up().with_shift());

        InputBindings(map)
    }
}

/// Label for systems that update bound input data. Runs in PreUpdate after InputSystem.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputBindingSystem;

fn clear_binding_input(mut binding_input: ResMut<ButtonInput<Binding>>) {
    binding_input.reset_all();
}

fn process_binding_input(
    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    bind_map: Res<InputBindings>,
    mut scroll_input: EventReader<MouseWheel>,
    mut bind_input: ResMut<ButtonInput<Binding>>,
) {
    bind_input.clear();

    // Collect scroll events.
    let last_scroll = scroll_input.read().last();

    for (bind, bind_val) in bind_map.iter() {
        let ctrl = kb_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = kb_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let alt = kb_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let main_just_pressed = match bind_val.source {
            BoundInputSource::Keyboard(key_code) => kb_input.just_pressed(key_code),
            BoundInputSource::Mouse(mouse_button) => mouse_input.just_pressed(mouse_button),
            BoundInputSource::ScrollUp => last_scroll.is_some_and(|scroll| scroll.y > 0.0),
            BoundInputSource::ScrollDown => last_scroll.is_some_and(|scroll| scroll.y < 0.0),
        };
        if ctrl == bind_val.ctrl
            && shift == bind_val.shift
            && alt == bind_val.alt
            && main_just_pressed
        {
            bind_input.press(*bind);
        }

        let main_just_released = match bind_val.source {
            BoundInputSource::Keyboard(key_code) => kb_input.just_released(key_code),
            BoundInputSource::Mouse(mouse_button) => mouse_input.just_released(mouse_button),
            BoundInputSource::ScrollUp => last_scroll.is_none(),
            BoundInputSource::ScrollDown => last_scroll.is_none(),
        };
        if main_just_released {
            bind_input.release(*bind);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<InputBindings>()
        .init_resource::<ButtonInput<Binding>>()
        .configure_sets(PreUpdate, InputBindingSystem.after(InputSystem))
        .add_systems(
            PreUpdate,
            (
                process_binding_input.in_set(InputBindingSystem),
                clear_binding_input
                    .before(process_binding_input)
                    .run_if(resource_exists_and_changed::<InputBindings>),
            ),
        );
}
