use avian3d::math::Quaternion;
use bevy::{
    prelude::*,
    state::state::FreelyMutableState,
    window::{CursorGrabMode, PrimaryWindow},
};
use serde::{Deserialize, Serialize};
use ulid::{serde::ulid_as_u128, Ulid};

pub trait FromPitchYawRoll {
    fn from_pitch_yaw_roll(pitch: f32, yaw: f32, roll: f32) -> Self;
}

impl FromPitchYawRoll for Quat {
    fn from_pitch_yaw_roll(pitch: f32, yaw: f32, roll: f32) -> Quat {
        Quaternion::from_euler(EulerRot::YXZ, pitch, yaw, roll)
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Facing3d {
    X,
    NegX,
    Y,
    NegY,
    Z,
    NegZ,
}

impl Facing3d {
    pub fn as_dir(&self) -> Dir3 {
        match self {
            Facing3d::X => Dir3::X,
            Facing3d::NegX => Dir3::NEG_X,
            Facing3d::Y => Dir3::Y,
            Facing3d::NegY => Dir3::NEG_Y,
            Facing3d::Z => Dir3::Z,
            Facing3d::NegZ => Dir3::NEG_Z,
        }
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
    move |inputs: Res<ButtonInput<T>>| inputs.just_pressed(input) || inputs.just_released(input)
}

/// Persistent identifier.
#[derive(
    Deref, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize,
)]
pub struct Id(#[serde(with = "ulid_as_u128")] pub Ulid);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Resource)]
pub struct IdGen(ulid::Generator);

impl Default for IdGen {
    fn default() -> Self {
        IdGen(ulid::Generator::new())
    }
}

impl IdGen {
    pub fn generate(&mut self) -> Id {
        Id(self.0.generate().unwrap())
    }
}
