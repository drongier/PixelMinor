use bevy::prelude::*;
use rand::Rng;

use crate::world::tiles::{
    GridPos, Tile, TileMinedEvent, TileType, BASE_RADIUS, GRID_SIZE, HIDDEN_COLOR, TILE_SIZE,
};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TileMinedEvent>()
            .add_systems(Startup, spawn_map)
            .add_systems(Update, reveal_on_mine);
    }
}

/// Tables de probabilité par zone (GDD)
fn pick_tile_type(distance: i32, rng: &mut impl Rng) -> TileType {
    let roll: f32 = rng.gen();

    match distance {
        3..=20 => {
            // Zone 1 : 70% Terre, 20% Pierre, 8% Charbon, 2% Fer
            if      roll < 0.70 { TileType::Dirt }
            else if roll < 0.90 { TileType::Stone }
            else if roll < 0.98 { TileType::Coal }
            else                { TileType::Iron }
        }
        21..=45 => {
            // Zone 2 : 30% Terre, 40% Pierre, 5% Charbon, 15% Fer, 10% Cuivre
            if      roll < 0.30 { TileType::Dirt }
            else if roll < 0.70 { TileType::Stone }
            else if roll < 0.75 { TileType::Coal }
            else if roll < 0.90 { TileType::Iron }
            else                { TileType::Copper }
        }
        46..=75 => {
            // Zone 3 : 50% Pierre, 20% Fer, 5% Cuivre, 15% Argent, 10% Or
            if      roll < 0.50 { TileType::Stone }
            else if roll < 0.70 { TileType::Iron }
            else if roll < 0.75 { TileType::Copper }
            else if roll < 0.90 { TileType::Silver }
            else                { TileType::Gold }
        }
        76..=110 => {
            // Zone 4 : 40% Pierre, 20% Argent, 15% Or, 15% Diamant, 10% Rubis
            if      roll < 0.40 { TileType::Stone }
            else if roll < 0.60 { TileType::Silver }
            else if roll < 0.75 { TileType::Gold }
            else if roll < 0.90 { TileType::Diamond }
            else                { TileType::Ruby }
        }
        _ => {
            // Zone 5 : 30% Pierre, 25% Obsidienne, 20% Mythril, 15% Diamant, 10% Rubis
            if      roll < 0.30 { TileType::Stone }
            else if roll < 0.55 { TileType::Obsidian }
            else if roll < 0.75 { TileType::Mythril }
            else if roll < 0.90 { TileType::Diamond }
            else                { TileType::Ruby }
        }
    }
}

fn spawn_map(mut commands: Commands) {
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

            commands.spawn((
                Tile { tile_type, hp, revealed },
                grid_pos,
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, -1.0),
            ));
        }
    }
}

/// Quand une tile est minée, révèle les 4 voisins
fn reveal_on_mine(
    mut events: EventReader<TileMinedEvent>,
    mut tile_query: Query<(&GridPos, &mut Tile, &mut Sprite)>,
) {
    for event in events.read() {
        let neighbor_positions = event.pos.neighbors();

        for (grid_pos, mut tile, mut sprite) in tile_query.iter_mut() {
            if !tile.revealed && neighbor_positions.contains(grid_pos) {
                tile.revealed = true;
                sprite.color = tile.tile_type.color();
            }
        }
    }
}
