use bevy::prelude::*;

use crate::economy::resources::{Inventory, PlayerStats, ShopOpen};
use crate::world::tiles::{GridPos, Tile, TileMinedEvent, TILE_SIZE};

const PLAYER_HALF: f32 = 2.0;
const TILE_HALF: f32 = TILE_SIZE / 2.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, player_movement)
            .add_systems(PostUpdate, camera_follow);
    }
}

#[derive(Component)]
pub struct Player;

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::splat(4.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

/// Retourne l'Entity de la tile qui bloque à cette position, s'il y en a une
fn blocking_tile(pos: Vec2, tiles: &[(Entity, Vec2)]) -> Option<Entity> {
    for &(entity, tile_pos) in tiles {
        let overlap_x = (pos.x - tile_pos.x).abs() < PLAYER_HALF + TILE_HALF;
        let overlap_y = (pos.y - tile_pos.y).abs() < PLAYER_HALF + TILE_HALF;
        if overlap_x && overlap_y {
            return Some(entity);
        }
    }
    None
}

fn player_movement(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    shop_open: Res<ShopOpen>,
    stats: Res<PlayerStats>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut tile_query: Query<(Entity, &Transform, &mut Tile, &GridPos), Without<Player>>,
    mut inventory: ResMut<Inventory>,
    mut mine_events: EventWriter<TileMinedEvent>,
) {
    if shop_open.0 {
        return;
    }
    let mut transform = player_query.single_mut();
    let mut direction = Vec2::ZERO;

    if keys.pressed(KeyCode::KeyZ) || keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction == Vec2::ZERO {
        return;
    }
    direction = direction.normalize();

    let current = transform.translation.truncate();
    let delta = direction * stats.walk_speed * time.delta_secs();

    // Collecte les tiles proches (entity + position)
    let nearby: Vec<(Entity, Vec2)> = tile_query
        .iter()
        .filter_map(|(e, t, _, _)| {
            let tp = t.translation.truncate();
            if current.distance(tp) < TILE_SIZE * 3.0 {
                Some((e, tp))
            } else {
                None
            }
        })
        .collect();

    // Test X : bouge ou mine
    let new_x = Vec2::new(current.x + delta.x, current.y);
    if let Some(blocked_entity) = blocking_tile(new_x, &nearby) {
        mine_tile(&mut commands, &mut tile_query, blocked_entity, &stats, &time, &mut inventory, &mut mine_events);
    } else {
        transform.translation.x = new_x.x;
    }

    // Test Y : bouge ou mine
    let new_y = Vec2::new(transform.translation.x, current.y + delta.y);
    if let Some(blocked_entity) = blocking_tile(new_y, &nearby) {
        mine_tile(&mut commands, &mut tile_query, blocked_entity, &stats, &time, &mut inventory, &mut mine_events);
    } else {
        transform.translation.y = new_y.y;
    }
}

fn mine_tile(
    commands: &mut Commands,
    tile_query: &mut Query<(Entity, &Transform, &mut Tile, &GridPos), Without<Player>>,
    entity: Entity,
    stats: &PlayerStats,
    time: &Time,
    inventory: &mut Inventory,
    mine_events: &mut EventWriter<TileMinedEvent>,
) {
    if let Ok((_, _, mut tile, grid_pos)) = tile_query.get_mut(entity) {
        tile.hp -= stats.mining_power * time.delta_secs();
        if tile.hp <= 0.0 {
            if inventory.total_count() < stats.inventory_capacity {
                *inventory.items.entry(tile.tile_type.clone()).or_insert(0) += 1;
            }
            mine_events.send(TileMinedEvent { pos: *grid_pos });
            commands.entity(entity).despawn();
        }
    }
}

fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let player_pos = player_query.single().translation;
    let mut camera = camera_query.single_mut();
    let speed = 5.0;
    camera.translation.x += (player_pos.x - camera.translation.x) * speed * 0.016;
    camera.translation.y += (player_pos.y - camera.translation.y) * speed * 0.016;
}
