use bevy::{
    prelude::*,
    state::state::FreelyMutableState,
    window::{CursorGrabMode, PrimaryWindow},
};
use ulid::{Generator, Ulid};

#[derive(Resource)]
pub struct IdGen(ulid::Generator);

impl Default for IdGen {
    fn default() -> Self {
        IdGen(ulid::Generator::new())
    }
}

impl IdGen {
    pub fn generate(&mut self) -> Ulid {
        self.0.generate().unwrap()
    }
}

pub fn enter_state<S: FreelyMutableState>(new_state: S) -> impl Fn(ResMut<NextState<S>>) {
    move |mut next_state| {
        next_state.set(new_state.clone());
    }
}

pub fn grab_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

pub fn release_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

pub fn input_just_toggled<T>(input: T) -> impl FnMut(Res<ButtonInput<T>>) -> bool + Clone
where
    T: Copy + Eq + core::hash::Hash + Send + Sync + 'static,
{
    move |inputs: Res<ButtonInput<T>>| inputs.just_released(input) || inputs.just_released(input)
}
