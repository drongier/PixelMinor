use bevy::prelude::*;

use crate::economy::resources::{GameState, Inventory, PlayerStats};
use crate::world::tiles::{GridPos, Tile, TileMap, TileMinedEvent, GRID_SIZE, TILE_SIZE};

const PLAYER_HALF: f32 = 2.0;
const TILE_HALF: f32 = TILE_SIZE / 2.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, player_movement.run_if(in_state(GameState::Playing)))
            .add_systems(
                PostUpdate,
                // Avant la propagation : sinon la caméra affiche la position de la frame précédente
                camera_follow.before(TransformSystem::TransformPropagate),
            );
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

type TileQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform, &'static mut Tile, &'static GridPos), Without<Player>>;

/// Bornes monde (min, max) du centre du joueur : il reste à l'intérieur de la grille
/// (cases -half..half-1, chaque case centrée sur `x * TILE_SIZE`).
fn player_bounds() -> (f32, f32) {
    let half = GRID_SIZE / 2;
    let min = -(half as f32) * TILE_SIZE - TILE_HALF + PLAYER_HALF;
    let max = (half - 1) as f32 * TILE_SIZE + TILE_HALF - PLAYER_HALF;
    (min, max)
}

#[allow(
    clippy::too_many_arguments,
    reason = "système Bevy : chaque paramètre est une ressource/query distincte"
)]
fn player_movement(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    stats: Res<PlayerStats>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut tile_query: TileQuery,
    tile_map: Res<TileMap>,
    mut inventory: ResMut<Inventory>,
    mut mine_events: EventWriter<TileMinedEvent>,
) {
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

    // Collecte les tiles proches (entity + position) via le TileMap, sur un carré 5x5
    let center = GridPos::from_world(current);
    let mut nearby: Vec<(Entity, Vec2)> = Vec::with_capacity(25);
    for dx in -2..=2 {
        for dy in -2..=2 {
            let pos = GridPos { x: center.x + dx, y: center.y + dy };
            let Some(&entity) = tile_map.tiles.get(&pos) else {
                continue;
            };
            // Err si la tile a déjà été despawnée
            if let Ok((_, t, _, _)) = tile_query.get(entity) {
                nearby.push((entity, t.translation.truncate()));
            }
        }
    }

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

    // Empêche de sortir de la carte après avoir miné une tile de bordure
    let (min, max) = player_bounds();
    transform.translation.x = transform.translation.x.clamp(min, max);
    transform.translation.y = transform.translation.y.clamp(min, max);
}

/// Mine la tile bloquante. Sac plein : la tile bloque simplement (aucune perte de HP),
/// pour ne jamais détruire une ressource qui ne peut pas être ramassée.
fn mine_tile(
    commands: &mut Commands,
    tile_query: &mut TileQuery,
    entity: Entity,
    stats: &PlayerStats,
    time: &Time,
    inventory: &mut Inventory,
    mine_events: &mut EventWriter<TileMinedEvent>,
) {
    if inventory.is_full(stats) {
        return;
    }
    if let Ok((_, _, mut tile, grid_pos)) = tile_query.get_mut(entity) {
        tile.hp -= stats.mining_power * time.delta_secs();
        if tile.hp <= 0.0 {
            *inventory.items.entry(tile.tile_type.clone()).or_insert(0) += 1;
            mine_events.send(TileMinedEvent { pos: *grid_pos });
            commands.entity(entity).despawn();
        }
    }
}

const CAMERA_FOLLOW_SPEED: f32 = 5.0;

fn camera_follow(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let player_pos = player_query.single().translation;
    let mut camera = camera_query.single_mut();
    // Lissage exponentiel indépendant du framerate
    let t = 1.0 - (-CAMERA_FOLLOW_SPEED * time.delta_secs()).exp();
    camera.translation.x += (player_pos.x - camera.translation.x) * t;
    camera.translation.y += (player_pos.y - camera.translation.y) * t;
}
