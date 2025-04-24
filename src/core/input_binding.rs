use bevy::input::mouse::MouseWheel;
use bevy::input::InputSystem;
use bevy::{prelude::*, utils::HashMap};
use itertools::Itertools;

/// Inputs bound to application actions.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum Binding {
    // Universal
    Quit,
    Playtest,
    Undo,
    Redo,

    // Movement
    MoveLeft,
    MoveRight,
    MoveBackwards,
    MoveForwards,
    MoveDown,
    MoveUp,
    Jump,

    // Camera
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BindingAxis {
    MoveX,
    MoveY,
    MoveZ,
    LookX,
    LookY,
}

pub trait BindingAxisFns {
    fn movement_vec(&self) -> Vec3;
    fn look_vec(&self) -> Vec2;
}

impl BindingAxisFns for Axis<BindingAxis> {
    /// Vector for movement.
    /// X is sideways, Y is up/down, Z is backwards/forwards.
    /// Remember: forwards is negative Z!
    fn movement_vec(&self) -> Vec3 {
        Vec3::new(
            self.get(BindingAxis::MoveX).unwrap_or(0.0),
            self.get(BindingAxis::MoveY).unwrap_or(0.0),
            self.get(BindingAxis::MoveZ).unwrap_or(0.0),
        )
        .normalize_or_zero()
    }

    /// Vector for looking. X = yaw, Y = pitch
    fn look_vec(&self) -> Vec2 {
        Vec2::new(
            self.get(BindingAxis::LookX).unwrap_or(0.0),
            self.get(BindingAxis::LookY).unwrap_or(0.0),
        )
        .normalize_or_zero()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BoundInput {
    pub source: BoundInputSource,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundInputSource {
    Keyboard(KeyCode),
    Mouse(MouseButton),
    ScrollUp,
    ScrollDown,
}

impl BoundInput {
    pub fn modifier_permutations(&self) -> Vec<BoundInput> {
        // cant figure out how to use this well
        vec![
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, false),
            (false, true, true),
            (true, false, true),
            (true, true, true),
        ]
        .into_iter()
        .map(|(ctrl, shift, alt)| BoundInput {
            source: self.source.clone(),
            ctrl,
            shift,
            alt,
        })
        .filter(|bound| bound != self)
        .collect()
    }

    pub fn key(keycode: KeyCode) -> Self {
        Self {
            source: BoundInputSource::Keyboard(keycode),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn mouse(button: MouseButton) -> Self {
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
pub struct InputBindingMap(HashMap<Binding, BoundInput>);

impl Default for InputBindingMap {
    fn default() -> Self {
        let mut map = HashMap::<Binding, BoundInput>::new();

        map.insert(Binding::Quit, BoundInput::key(KeyCode::KeyQ).with_control());
        map.insert(Binding::Playtest, BoundInput::key(KeyCode::Tab));
        map.insert(Binding::Undo, BoundInput::key(KeyCode::KeyZ).with_control());
        map.insert(
            Binding::Redo,
            BoundInput::key(KeyCode::KeyZ).with_control().with_shift(),
        );
        map.insert(Binding::MoveLeft, BoundInput::key(KeyCode::KeyA));
        map.insert(Binding::MoveRight, BoundInput::key(KeyCode::KeyD));
        map.insert(Binding::MoveBackwards, BoundInput::key(KeyCode::KeyS));
        map.insert(Binding::MoveForwards, BoundInput::key(KeyCode::KeyW));
        map.insert(Binding::MoveDown, BoundInput::key(KeyCode::KeyQ));
        map.insert(Binding::MoveUp, BoundInput::key(KeyCode::KeyE));
        map.insert(Binding::Jump, BoundInput::key(KeyCode::Space));
        map.insert(Binding::LookLeft, BoundInput::key(KeyCode::ArrowLeft));
        map.insert(Binding::LookRight, BoundInput::key(KeyCode::ArrowRight));
        map.insert(Binding::LookDown, BoundInput::key(KeyCode::ArrowDown));
        map.insert(Binding::LookUp, BoundInput::key(KeyCode::ArrowUp));
        map.insert(Binding::FlyMode, BoundInput::mouse(MouseButton::Right));
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
            BoundInput::mouse(MouseButton::Right).with_shift(),
        );
        map.insert(
            Binding::ResetSelAxisOffset,
            BoundInput::key(KeyCode::Digit0),
        );
        map.insert(Binding::ToggleSnap, BoundInput::key(KeyCode::KeyT));
        map.insert(Binding::HoldSnap, BoundInput::key(KeyCode::AltLeft));
        map.insert(Binding::SelNext, BoundInput::scroll_down().with_shift());
        map.insert(Binding::SelPrev, BoundInput::scroll_up().with_shift());

        InputBindingMap(map)
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
    bind_map: Res<InputBindingMap>,
    mut scroll_input: EventReader<MouseWheel>,
    mut bind_input: ResMut<ButtonInput<Binding>>,
) {
    bind_input.bypass_change_detection().clear();

    // Collect scroll events.
    let last_scroll = scroll_input.read().last();

    for (binding, bound_input) in bind_map.iter() {
        let ctrl = kb_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = kb_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let alt = kb_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let main_just_pressed = match bound_input.source {
            BoundInputSource::Keyboard(key_code) => kb_input.just_pressed(key_code),
            BoundInputSource::Mouse(mouse_button) => mouse_input.just_pressed(mouse_button),
            BoundInputSource::ScrollUp => last_scroll.is_some_and(|scroll| scroll.y > 0.0),
            BoundInputSource::ScrollDown => last_scroll.is_some_and(|scroll| scroll.y < 0.0),
        };
        if ctrl == bound_input.ctrl
            && shift == bound_input.shift
            && alt == bound_input.alt
            && main_just_pressed
        {
            bind_input.press(*binding);
        }

        let main_just_released = match bound_input.source {
            BoundInputSource::Keyboard(key_code) => kb_input.just_released(key_code),
            BoundInputSource::Mouse(mouse_button) => mouse_input.just_released(mouse_button),
            BoundInputSource::ScrollUp => last_scroll.is_none(),
            BoundInputSource::ScrollDown => last_scroll.is_none(),
        };

        // TODO: Maybe itshould release for example W when Shift+W is now pressed (so cancel
        // eariler presses of same key but different modifiers)
        // When pressing binding, find all permutations of same source and release..
        if main_just_released {
            bind_input.release(*binding);
        }
    }
}

fn binding_input_to_axes(
    bind_input: Res<ButtonInput<Binding>>,
    mut axis_input: ResMut<Axis<BindingAxis>>,
) {
    axis_input.set(
        BindingAxis::MoveX,
        f32::from(bind_input.pressed(Binding::MoveRight))
            - f32::from(bind_input.pressed(Binding::MoveLeft)),
    );
    axis_input.set(
        BindingAxis::MoveY,
        f32::from(bind_input.pressed(Binding::MoveUp))
            - f32::from(bind_input.pressed(Binding::MoveDown)),
    );
    axis_input.set(
        BindingAxis::MoveZ,
        f32::from(bind_input.pressed(Binding::MoveBackwards))
            - f32::from(bind_input.pressed(Binding::MoveForwards)),
    );
    axis_input.set(
        BindingAxis::LookX,
        f32::from(bind_input.pressed(Binding::LookRight))
            - f32::from(bind_input.pressed(Binding::LookLeft)),
    );
    axis_input.set(
        BindingAxis::LookY,
        f32::from(bind_input.pressed(Binding::LookUp))
            - f32::from(bind_input.pressed(Binding::LookDown)),
    );
}

pub fn plugin(app: &mut App) {
    app.init_resource::<InputBindingMap>()
        .init_resource::<ButtonInput<Binding>>()
        .init_resource::<Axis<BindingAxis>>()
        .configure_sets(PreUpdate, InputBindingSystem.after(InputSystem))
        .add_systems(
            PreUpdate,
            (
                (process_binding_input, binding_input_to_axes)
                    .chain()
                    .in_set(InputBindingSystem),
                clear_binding_input
                    .before(process_binding_input)
                    .run_if(resource_exists_and_changed::<InputBindingMap>),
            ),
        );
}
