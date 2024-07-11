use bevy::math::Vec3;

pub fn move_toward_3d(from: Vec3, to: Vec3, delta: f32) -> Vec3 {
    let diff = to - from;
    let length = diff.length();
    if length <= delta {
        to
    } else {
        from + diff / length * delta
    }
}
