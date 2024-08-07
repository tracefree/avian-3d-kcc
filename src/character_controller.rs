use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_debug_text_overlay::screen_print;

use crate::schedule::CustomPostUpdate;

const MAX_BOUNCES: usize = 4;
const MAX_CLIP_PLANES: usize = 5;
const SKIN_WIDTH: f32 = 0.005;

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub struct CharacterControllerSet;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            CustomPostUpdate,
            move_character_controllers.in_set(CharacterControllerSet),
        );
    }
}

#[derive(Component, Default)]
pub struct CharacterController {
    pub velocity: Vec3, // todo: this is a Vec3 but do we support vertical movement?
}

fn move_character_controllers(
    mut query: Query<(Entity, &CharacterController, &Collider, &mut Transform)>,
    spatial_query: SpatialQuery,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    for (entity, character_controller, collider, mut transform) in &mut query {
        let mut direction_result = Dir3::new(character_controller.velocity);
        let mut distance = character_controller.velocity.length() * time.delta_seconds();

        let Ok(start_direction) = direction_result else {
            continue;
        };

        let mut bounce_count = 0;
        let mut hit_count = 0;

        let mut num_planes = 0;
        let mut planes = [Vec3::ZERO; MAX_CLIP_PLANES];

        'bounce_loop: for _ in 0..MAX_BOUNCES {
            bounce_count += 1;

            if let Ok(direction) = direction_result {
                gizmos.ray(
                    transform.translation,
                    direction.as_vec3(),
                    Color::linear_rgb(1.0, 0.0, 0.0),
                );

                if let Some(hit) = spatial_query.cast_shape(
                    collider,
                    transform.translation,
                    transform.rotation,
                    direction,
                    distance + SKIN_WIDTH,
                    true,
                    SpatialQueryFilter::from_excluded_entities([entity]),
                ) {
                    hit_count += 1;

                    screen_print!("normal: {}", hit.normal1);

                    let hit_point = *transform * hit.point2;

                    gizmos.sphere(hit_point, Quat::IDENTITY, 0.1, Color::WHITE);

                    if hit.time_of_impact >= distance {
                        transform.translation +=
                            direction * (hit.time_of_impact - SKIN_WIDTH).max(0.0);
                        break;
                    }

                    transform.translation += direction * (hit.time_of_impact - SKIN_WIDTH);

                    // If we move above a threshold, consider previous planes as no longer active obstacles.
                    if (hit.time_of_impact - SKIN_WIDTH).abs() > 0.01 {
                        num_planes = 0;
                    }

                    // Too many obstacles, let's give up.
                    if num_planes >= MAX_CLIP_PLANES {
                        break;
                    }

                    // Add the obstacle we hit to the sliding planes.
                    planes[num_planes] = hit.normal1;
                    num_planes += 1;

                    let extra_distance = distance - (hit.time_of_impact - SKIN_WIDTH).max(0.0);
                    let extra_velocity = direction * extra_distance;

                    // Inspired by Quake's collision resolution
                    let mut projected_velocity = extra_velocity;
                    let mut walk_along_crease = false;
                    'clip_planes: for i in 0..num_planes {
                        projected_velocity = extra_velocity.reject_from_normalized(planes[i]);
                        for j in 0..num_planes {
                            if j != i && projected_velocity.dot(planes[j]) < 0.0 {
                                walk_along_crease = true;
                                break 'clip_planes;
                            }
                        }
                    }

                    if walk_along_crease {
                        if num_planes != 2 {
                            // Not sure about this...
                            break 'bounce_loop;
                        }
                        let crease_direction = planes[0].cross(planes[1]);
                        projected_velocity =
                            crease_direction * crease_direction.dot(projected_velocity);
                    }

                    // Avoid moving backwards
                    if projected_velocity.dot(*start_direction) <= 0.0 {
                        break;
                    }

                    direction_result = Dir3::new(projected_velocity);
                    distance = projected_velocity.length();
                } else {
                    transform.translation += direction * distance;
                    break;
                }
            } else {
                break;
            }
        }

        screen_print!("bounces: {}", bounce_count);
        screen_print!("hit count: {}", hit_count);
    }
}
