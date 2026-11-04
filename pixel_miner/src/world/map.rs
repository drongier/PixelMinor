use bevy::prelude::*;
use rand::Rng;

use crate::world::tiles::{
    GridPos, Tile, TileMap, TileMinedEvent, TileType, Zone, BASE_RADIUS, GRID_SIZE, HIDDEN_COLOR,
    TILE_SIZE,
};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TileMinedEvent>()
            .init_resource::<TileMap>()
            .add_systems(Startup, spawn_map)
            .add_systems(Update, reveal_on_mine);
    }
}

/// Tables de probabilité par zone (GDD)
fn pick_tile_type(distance: i32, rng: &mut impl Rng) -> TileType {
    let roll: f32 = rng.gen();

    match Zone::from_distance(distance) {
        // La base n'a pas de tiles : jamais appelé avec Base
        Zone::Base | Zone::Z1 => {
            // Zone 1 : 70% Terre, 20% Pierre, 8% Charbon, 2% Fer
            if      roll < 0.70 { TileType::Dirt }
            else if roll < 0.90 { TileType::Stone }
            else if roll < 0.98 { TileType::Coal }
            else                { TileType::Iron }
        }
        Zone::Z2 => {
            // Zone 2 : 30% Terre, 40% Pierre, 5% Charbon, 15% Fer, 10% Cuivre
            if      roll < 0.30 { TileType::Dirt }
            else if roll < 0.70 { TileType::Stone }
            else if roll < 0.75 { TileType::Coal }
            else if roll < 0.90 { TileType::Iron }
            else                { TileType::Copper }
        }
        Zone::Z3 => {
            // Zone 3 : 50% Pierre, 20% Fer, 5% Cuivre, 15% Argent, 10% Or
            if      roll < 0.50 { TileType::Stone }
            else if roll < 0.70 { TileType::Iron }
            else if roll < 0.75 { TileType::Copper }
            else if roll < 0.90 { TileType::Silver }
            else                { TileType::Gold }
        }
        Zone::Z4 => {
            // Zone 4 : 40% Pierre, 20% Argent, 15% Or, 15% Diamant, 10% Rubis
            if      roll < 0.40 { TileType::Stone }
            else if roll < 0.60 { TileType::Silver }
            else if roll < 0.75 { TileType::Gold }
            else if roll < 0.90 { TileType::Diamond }
            else                { TileType::Ruby }
        }
        Zone::Z5 => {
            // Zone 5 : 30% Pierre, 25% Obsidienne, 20% Mythril, 15% Diamant, 10% Rubis
            if      roll < 0.30 { TileType::Stone }
            else if roll < 0.55 { TileType::Obsidian }
            else if roll < 0.75 { TileType::Mythril }
            else if roll < 0.90 { TileType::Diamond }
            else                { TileType::Ruby }
        }
    }
}

fn spawn_map(mut commands: Commands, mut tile_map: ResMut<TileMap>) {
    let half = GRID_SIZE / 2;
    let mut rng = rand::thread_rng();

    for x in -half..half {
        for y in -half..half {
            let grid_pos = GridPos { x, y };
            let dist = grid_pos.distance_from_center();

            if dist <= BASE_RADIUS {
                continue;
            }

            let tile_type = pick_tile_type(dist, &mut rng);
            let hp = tile_type.hp();

            // Tiles adjacentes à la base sont révélées au départ
            let revealed = dist <= BASE_RADIUS + 1;
            let color = if revealed { tile_type.color() } else { HIDDEN_COLOR };

            let world = grid_pos.to_world();

            let entity = commands.spawn((
                Tile { tile_type, hp, revealed },
                grid_pos,
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, -1.0),
            )).id();
            tile_map.tiles.insert(grid_pos, entity);
        }
    }
}

/// Quand une tile est minée : la retire du TileMap et révèle les 4 voisins.
/// Le despawn est fait par le mineur via Commands ; une entrée encore présente
/// mais déjà despawnée est sans danger car `get_mut` renvoie Err.
fn reveal_on_mine(
    mut events: EventReader<TileMinedEvent>,
    mut tile_map: ResMut<TileMap>,
    mut tile_query: Query<(&mut Tile, &mut Sprite)>,
) {
    for event in events.read() {
        tile_map.tiles.remove(&event.pos);

        for neighbor in event.pos.neighbors() {
            let Some(&entity) = tile_map.tiles.get(&neighbor) else {
                continue;
            };
            if let Ok((mut tile, mut sprite)) = tile_query.get_mut(entity) {
                if !tile.revealed {
                    tile.revealed = true;
                    sprite.color = tile.tile_type.color();
                }
            }
        }
    }
}
