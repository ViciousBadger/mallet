use bevy::{
    prelude::*,
    state::state::FreelyMutableState,
    window::{CursorGrabMode, PrimaryWindow},
};

pub fn enter_state<S: FreelyMutableState>(new_state: S) -> impl Fn(ResMut<NextState<S>>) {
    move |mut next_state| next_state.set(new_state.clone())
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
