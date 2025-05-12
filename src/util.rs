use avian3d::math::Quaternion;
use bevy::{
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    prelude::*,
    state::state::FreelyMutableState,
    window::{CursorGrabMode, PrimaryWindow},
};

pub trait FromPitchYawRoll {
    fn from_pitch_yaw_roll(pitch: f32, yaw: f32, roll: f32) -> Self;
}

impl FromPitchYawRoll for Quat {
    fn from_pitch_yaw_roll(pitch: f32, yaw: f32, roll: f32) -> Quat {
        Quaternion::from_euler(EulerRot::YXZ, pitch, yaw, roll)
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
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

pub fn grab_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut window = q_window.single_mut()?;
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
    Ok(())
}

pub fn release_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut window = q_window.single_mut()?;
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
    Ok(())
}

pub fn input_just_toggled<T>(input: T) -> impl FnMut(Res<ButtonInput<T>>) -> bool + Clone
where
    T: Copy + Eq + core::hash::Hash + Send + Sync + 'static,
{
    move |inputs: Res<ButtonInput<T>>| inputs.just_pressed(input) || inputs.just_released(input)
}

pub fn brush_texture_settings(settings: &mut ImageLoaderSettings) {
    *settings = ImageLoaderSettings {
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        }),
        ..default()
    }
}

/// This is possibly a bad idea, for more complex state matching
pub fn test_state<S: States>(
    f: impl Fn(S) -> bool + Clone,
) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => f(current_state.clone()),
        None => false,
    }
}
