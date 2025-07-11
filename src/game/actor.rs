use avian3d::prelude::*;
use bevy::{math::vec2, prelude::*};

use crate::{
    core::binds::{Binding, BindingAxis, BindingAxisFns, InputBindingSystem},
    game::GameSystems,
};

#[derive(Component)]
pub struct PlayerActor;

#[derive(Component)]
struct Grounded;

#[derive(Event)]
pub enum MovementAction {
    Move(Vec2),
    Jump,
}

fn player_input(
    button_input: Res<ButtonInput<Binding>>,
    axis_input: Res<Axis<BindingAxis>>,
    mut movement_event_writer: EventWriter<MovementAction>,
) {
    let movement = axis_input.movement_vec().xz();
    if movement != Vec2::ZERO {
        movement_event_writer.write(MovementAction::Move(movement));
    }
    if button_input.just_pressed(Binding::Jump) {
        movement_event_writer.write(MovementAction::Jump);
    }
}

fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), With<PlayerActor>>,
) {
    for (entity, hits, rotation) in query.iter_mut() {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let max_slope_angle = 30.0_f32.to_radians();
        let is_grounded = hits
            .iter()
            .any(|hit| (rotation * -hit.normal2).angle_between(Vec3::Y).abs() <= max_slope_angle);

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

const ACCEL: f32 = 60.0;
const AIR_ACCEL: f32 = 14.0;
const TOP_SPEED: f32 = 6.0;
const GRAV: f32 = 9.81 * 2.0;
const JUMP: f32 = 8.0;

fn apply_gravity(time: Res<Time>, mut q_actors: Query<&mut LinearVelocity, With<PlayerActor>>) {
    let gravity = Vec3::NEG_Y * GRAV;
    for mut linear_velocity in &mut q_actors.iter_mut() {
        linear_velocity.0 += gravity * time.delta_secs();
    }
}

fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(&GlobalTransform, &mut LinearVelocity, Has<Grounded>)>,
) {
    let delta_time = time.delta_secs();

    for event in movement_event_reader.read() {
        for (transform, mut linear_velocity, is_grounded) in &mut controllers {
            let accel = if is_grounded { ACCEL } else { AIR_ACCEL };
            match event {
                MovementAction::Move(direction) => {
                    let rot = Rot2::radians(-transform.rotation().to_euler(EulerRot::YXZ).0);
                    let rotated_dir = rot * *direction;

                    linear_velocity.x += rotated_dir.x * accel * delta_time;
                    linear_velocity.z += rotated_dir.y * accel * delta_time;
                }
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = JUMP;
                    }
                }
            }
        }
    }
}

fn apply_movement_damping(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut q_actors: Query<(&mut LinearVelocity, Has<Grounded>), With<PlayerActor>>,
) {
    let moving = movement_event_reader
        .read()
        .any(|event| matches!(event, MovementAction::Move(..)));

    for (mut linear_velocity, is_grounded) in q_actors.iter_mut() {
        let accel = if is_grounded { ACCEL } else { AIR_ACCEL };
        let dampened_xz = vec2(linear_velocity.x, linear_velocity.z)
            .move_towards(
                Vec2::ZERO,
                if moving {
                    0.0
                } else {
                    time.delta_secs() * accel
                },
            )
            .clamp_length_max(TOP_SPEED);

        **linear_velocity = Vec3::new(dampened_xz.x, linear_velocity.y, dampened_xz.y);
    }
}
/// Kinematic bodies do not get pushed by collisions by default,
/// so it needs to be done manually.
///
/// This system handles collision response for kinematic character controllers
/// by pushing them along their contact normals by the current penetration depth,
/// and applying velocity corrections in order to snap to slopes, slide along walls,
/// and predict collisions using speculative contacts.
#[allow(clippy::type_complexity)]
fn actor_collisions(
    collisions: Collisions,
    bodies: Query<&RigidBody>,
    collider_rbs: Query<&ColliderOf, Without<Sensor>>,
    mut character_controllers: Query<
        (&mut Position, &mut LinearVelocity),
        (With<RigidBody>, With<PlayerActor>),
    >,
    time: Res<Time>,
) {
    let max_slope_angle = 30.0_f32.to_radians();

    // Iterate through collisions and move the kinematic body to resolve penetration
    for contacts in collisions.iter() {
        // Get the rigid body entities of the colliders (colliders could be children)
        let Ok([&ColliderOf { body: rb1 }, &ColliderOf { body: rb2 }]) =
            collider_rbs.get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Get the body of the character controller and whether it is the first
        // or second entity in the collision.
        let is_first: bool;

        let character_rb: RigidBody;
        let is_other_dynamic: bool;

        let (mut position, mut linear_velocity) =
            if let Ok(character) = character_controllers.get_mut(rb1) {
                is_first = true;
                character_rb = *bodies.get(rb1).unwrap();
                is_other_dynamic = bodies.get(rb2).is_ok_and(|rb| rb.is_dynamic());
                character
            } else if let Ok(character) = character_controllers.get_mut(rb2) {
                is_first = false;
                character_rb = *bodies.get(rb2).unwrap();
                is_other_dynamic = bodies.get(rb1).is_ok_and(|rb| rb.is_dynamic());
                character
            } else {
                continue;
            };

        // This system only handles collision response for kinematic character controllers.
        if !character_rb.is_kinematic() {
            continue;
        }

        // Iterate through contact manifolds and their contacts.
        // Each contact in a single manifold shares the same contact normal.
        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.normal
            } else {
                manifold.normal
            };

            let mut deepest_penetration = f32::MIN;

            // Solve each penetrating contact in the manifold.
            for contact in manifold.points.iter() {
                if contact.penetration > 0.0 {
                    position.0 += normal * contact.penetration;
                }
                deepest_penetration = deepest_penetration.max(contact.penetration);
            }

            // For now, this system only handles velocity corrections for collisions against static geometry.
            if is_other_dynamic {
                continue;
            }

            // Determine if the slope is climbable or if it's too steep to walk on.
            let slope_angle = normal.angle_between(Vec3::Y);
            let climbable = slope_angle.abs() <= max_slope_angle;

            if deepest_penetration > 0.0 {
                // If the slope is climbable, snap the velocity so that the character
                // up and down the surface smoothly.
                if climbable {
                    // Points in the normal's direction in the XZ plane.
                    let normal_direction_xz =
                        normal.reject_from_normalized(Vec3::Y).normalize_or_zero();

                    // The movement speed along the direction above.
                    let linear_velocity_xz = linear_velocity.dot(normal_direction_xz);

                    // Snap the Y speed based on the speed at which the character is moving
                    // up or down the slope, and how steep the slope is.
                    //
                    // A 2D visualization of the slope, the contact normal, and the velocity components:
                    //
                    //             ╱
                    //     normal ╱
                    // *         ╱
                    // │   *    ╱   velocity_x
                    // │       * - - - - - -
                    // │           *       | velocity_y
                    // │               *   |
                    // *───────────────────*

                    let max_y_speed = -linear_velocity_xz * slope_angle.tan();
                    linear_velocity.y = linear_velocity.y.max(max_y_speed);
                } else {
                    // The character is intersecting an unclimbable object, like a wall.
                    // We want the character to slide along the surface, similarly to
                    // a collide-and-slide algorithm.

                    // Don't apply an impulse if the character is moving away from the surface.
                    if linear_velocity.dot(normal) > 0.0 {
                        continue;
                    }

                    // Slide along the surface, rejecting the velocity along the contact normal.
                    let impulse = linear_velocity.reject_from_normalized(normal);
                    linear_velocity.0 = impulse;
                }
            } else {
                // The character is not yet intersecting the other object,
                // but the narrow phase detected a speculative collision.
                //
                // We need to push back the part of the velocity
                // that would cause penetration within the next frame.

                let normal_speed = linear_velocity.dot(normal);

                // Don't apply an impulse if the character is moving away from the surface.
                if normal_speed > 0.0 {
                    continue;
                }

                // Compute the impulse to apply.
                let impulse_magnitude = normal_speed - (deepest_penetration / time.delta_secs());
                let mut impulse = impulse_magnitude * normal;

                // Apply the impulse differently depending on the slope angle.
                if climbable {
                    // Avoid sliding down slopes.
                    linear_velocity.y -= impulse.y.min(0.0);
                } else {
                    // Avoid climbing up walls.
                    impulse.y = impulse.y.max(0.0);
                    linear_velocity.0 -= impulse;
                }
            }
        }
    }
}
pub fn plugin(app: &mut App) {
    app.add_event::<MovementAction>()
        .add_systems(
            PreUpdate,
            player_input.after(InputBindingSystem).in_set(GameSystems),
        )
        .add_systems(
            Update,
            (
                update_grounded,
                apply_gravity,
                movement,
                apply_movement_damping,
            )
                .chain()
                .in_set(GameSystems),
        )
        .add_systems(
            PhysicsSchedule,
            actor_collisions.in_set(NarrowPhaseSet::Last),
        );
}
