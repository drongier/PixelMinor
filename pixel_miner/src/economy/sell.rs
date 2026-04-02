use bevy::prelude::*;

use crate::player::player::Player;
use crate::economy::resources::{Inventory, Wallet};
use crate::world::tiles::TILE_SIZE;

const SELL_ZONE_SIZE: f32 = TILE_SIZE * 2.0;

pub struct SellPlugin;

impl Plugin for SellPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_sell_zone)
            .add_systems(Update, sell_zone_system);
    }
}

#[derive(Component)]
struct SellZone;

fn spawn_sell_zone(mut commands: Commands) {
    commands.spawn((
        SellZone,
        Sprite {
            color: Color::srgb(0.0, 0.8, 0.4),
            custom_size: Some(Vec2::splat(SELL_ZONE_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, -40.0, -2.0),
    ));
}

fn sell_zone_system(
    player_query: Query<&Transform, With<Player>>,
    sell_query: Query<&Transform, With<SellZone>>,
    mut inventory: ResMut<Inventory>,
    mut wallet: ResMut<Wallet>,
) {
    let player_pos = player_query.single().translation.truncate();
    let sell_pos = sell_query.single().translation.truncate();
    let half = SELL_ZONE_SIZE / 2.0;

    let in_zone = (player_pos.x - sell_pos.x).abs() <= half
        && (player_pos.y - sell_pos.y).abs() <= half;

    if !in_zone || inventory.items.is_empty() {
        return;
    }

    let total: u64 = inventory
        .items
        .iter()
        .map(|(t, qty)| t.value() * (*qty as u64))
        .sum();

    wallet.money += total;
    inventory.items.clear();
}
