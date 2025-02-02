use std::time::Duration;

use bevy::{
    core::{Time, Timer},
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::Vec2,
    prelude::{
        default, App, AssetServer, Assets, Commands, Component, Entity, EventReader, Handle,
        Plugin, Query, Res, Transform, With,
    },
    sprite::SpriteBundle,
    transform::TransformBundle,
};
use bevy_rapier2d::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::{
    animation::Facing,
    audio::FighterStateEffectsPlayback,
    collisions::BodyLayers,
    consts::{ATTACK_HEIGHT, ATTACK_LAYER, ATTACK_WIDTH, THROW_ITEM_ROTATION_SPEED},
    input::PlayerAction,
    metadata::FighterMeta,
    movement::{MoveInDirection, Rotate, Target},
    state::State,
    ArrivedEvent, Enemy, GameState, Player,
};

pub struct AttackPlugin;

impl Plugin for AttackPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(player_attack.run_in_state(GameState::InGame));
    }
}

#[cfg_attr(feature = "debug", derive(bevy_inspector_egui::Inspectable))]
#[derive(Component)]
pub struct Attack {
    pub damage: i32,
}

#[derive(Component)]
pub struct AttackTimer(pub Timer);

fn player_attack(
    query: Query<(&Transform, &Facing, &State, &ActionState<PlayerAction>), With<Player>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (transform, facing, state, input) in query.iter() {
        if *state != State::Idle && *state != State::Running {
            break;
        }
        if input.just_pressed(PlayerAction::Shoot) {
            let mut dir = Vec2::X;

            if facing.is_left() {
                dir = -dir;
            }

            commands
                // .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
                //     transform.translation.x,
                //     transform.translation.y,
                //     ATTACK_LAYER,
                // )))
                .spawn_bundle(SpriteBundle {
                    texture: asset_server.load("bottled_seaweed11x31.png"),
                    transform: Transform::from_xyz(
                        transform.translation.x,
                        transform.translation.y,
                        ATTACK_LAYER,
                    ),
                    ..default()
                })
                .insert(Rotate {
                    speed: THROW_ITEM_ROTATION_SPEED,
                    to_right: !facing.is_left(),
                })
                .insert(Collider::cuboid(ATTACK_WIDTH / 2., ATTACK_HEIGHT / 2.))
                .insert(Sensor(true))
                .insert(ActiveEvents::COLLISION_EVENTS)
                .insert(ActiveCollisionTypes::default() | ActiveCollisionTypes::STATIC_STATIC)
                .insert(CollisionGroups::new(
                    BodyLayers::PLAYER_ATTACK,
                    BodyLayers::ENEMY,
                ))
                .insert(facing.clone())
                .insert(MoveInDirection(dir * 300.)) //TODO: Put the velocity in a const
                // .insert(Velocity::from_linear(dir * 300.))
                .insert(Attack { damage: 10 })
                .insert(AttackTimer(Timer::new(Duration::from_secs(1), false)));
        }
    }
}

pub fn enemy_attack(
    mut query: Query<(Entity, &mut State, &Handle<FighterMeta>), (With<Enemy>, With<Target>)>,
    mut event_reader: EventReader<ArrivedEvent>,
    mut commands: Commands,
    fighter_assets: Res<Assets<FighterMeta>>,
) {
    for event in event_reader.iter() {
        if let Ok((entity, mut state, fighter_handle)) = query.get_mut(event.0) {
            if *state != State::Attacking {
                if rand::random() && *state != State::Waiting {
                    state.set(State::Waiting);
                } else {
                    state.set(State::Attacking);

                    let attack_entity = commands
                        .spawn_bundle(TransformBundle::default())
                        .insert(Collider::cuboid(ATTACK_WIDTH * 0.8, ATTACK_HEIGHT * 0.8))
                        .insert(Sensor(true))
                        .insert(ActiveEvents::COLLISION_EVENTS)
                        .insert(
                            ActiveCollisionTypes::default() | ActiveCollisionTypes::STATIC_STATIC,
                        )
                        .insert(CollisionGroups::new(
                            BodyLayers::ENEMY_ATTACK,
                            BodyLayers::PLAYER,
                        ))
                        .insert(Attack { damage: 10 })
                        .insert(AttackTimer(Timer::new(
                            Duration::from_secs_f32(0.48),
                            false,
                        )))
                        .id();
                    commands.entity(event.0).push_children(&[attack_entity]);

                    if let Some(fighter) = fighter_assets.get(fighter_handle) {
                        if let Some(effects) = fighter.audio.effect_handles.get(&state) {
                            let fx_playback =
                                FighterStateEffectsPlayback::new(*state, effects.clone());
                            commands.entity(entity).insert(fx_playback);
                        }
                    }
                }
            }
        }
    }
}

pub fn attack_cleanup(query: Query<(Entity, &AttackTimer), With<Attack>>, mut commands: Commands) {
    for (entity, timer) in query.iter() {
        if timer.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn attack_tick(mut query: Query<&mut AttackTimer, With<Attack>>, time: Res<Time>) {
    for mut timer in query.iter_mut() {
        timer.0.tick(time.delta());
    }
}
