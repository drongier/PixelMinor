use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::world::deposits::{Deposit, DepositType};
use crate::player::player::Player;
use crate::economy::resources::{Inventory, PlayerStats, ShopOpen};
use crate::world::tiles::{GridPos, TileType, TILE_SIZE};

pub struct AutoMinerPlugin;

impl Plugin for AutoMinerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MinerInventory::default())
            .add_systems(Update, (handle_b_key, auto_miner_production));
    }
}

#[derive(Resource, Default)]
pub struct MinerInventory {
    pub count: u32,
}

#[derive(Component)]
pub struct AutoMiner {
    pub deposit_type: DepositType,
    pub storage: HashMap<TileType, u32>,
    pub timer: Timer,
}

const MINER_PROXIMITY: f32 = TILE_SIZE * 2.5;
const MAX_MINER_STORAGE: u32 = 50;
const PRODUCTION_INTERVAL: f32 = 10.0;

fn handle_b_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut miner_inv: ResMut<MinerInventory>,
    mut inventory: ResMut<Inventory>,
    stats: Res<PlayerStats>,
    player_query: Query<&Transform, With<Player>>,
    deposit_query: Query<(Entity, &GridPos, &Deposit), Without<AutoMiner>>,
    mut miner_query: Query<(Entity, &Transform, &mut AutoMiner)>,
    existing_miners: Query<&GridPos, With<AutoMiner>>,
) {
    if !keys.just_pressed(KeyCode::KeyB) || shop_open.0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();

    // Priorité 1 : placer un mineur si on en a et qu'un gisement libre est proche
    if miner_inv.count > 0 {
        let mut best: Option<(Entity, GridPos, DepositType, f32)> = None;

        for (entity, grid_pos, deposit) in deposit_query.iter() {
            if !deposit.revealed {
                continue;
            }

            // Vérifier qu'il n'y a pas déjà un mineur sur cette tile
            let already_has_miner = existing_miners.iter().any(|mp| *mp == *grid_pos);
            if already_has_miner {
                continue;
            }

            let world = grid_pos.to_world();
            let dist = player_pos.distance(world);
            if dist <= MINER_PROXIMITY {
                if best.is_none() || dist < best.as_ref().unwrap().3 {
                    best = Some((entity, *grid_pos, deposit.deposit_type.clone(), dist));
                }
            }
        }

        if let Some((_entity, grid_pos, deposit_type, _)) = best {
            miner_inv.count -= 1;
            let world = grid_pos.to_world();

            commands.spawn((
                AutoMiner {
                    deposit_type,
                    storage: HashMap::new(),
                    timer: Timer::from_seconds(PRODUCTION_INTERVAL, TimerMode::Repeating),
                },
                grid_pos,
                Sprite {
                    color: Color::srgb(0.9, 0.6, 0.1),
                    custom_size: Some(Vec2::splat(TILE_SIZE * 0.6)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, -0.5),
            ));
            return;
        }
    }

    // Priorité 2 : collecter depuis un mineur proche qui a du stock
    let mut best_miner: Option<(Entity, f32)> = None;
    for (entity, transform, miner) in miner_query.iter() {
        let total: u32 = miner.storage.values().sum();
        if total == 0 {
            continue;
        }
        let dist = player_pos.distance(transform.translation.truncate());
        if dist <= MINER_PROXIMITY {
            if best_miner.is_none() || dist < best_miner.as_ref().unwrap().1 {
                best_miner = Some((entity, dist));
            }
        }
    }

    if let Some((entity, _)) = best_miner {
        if let Ok((_, _, mut miner)) = miner_query.get_mut(entity) {
            let capacity_left = stats.inventory_capacity.saturating_sub(inventory.total_count());
            let mut transferred = 0u32;

            for (tile_type, qty) in miner.storage.iter_mut() {
                if transferred >= capacity_left {
                    break;
                }
                let can_take = (*qty).min(capacity_left - transferred);
                *inventory.items.entry(tile_type.clone()).or_insert(0) += can_take;
                *qty -= can_take;
                transferred += can_take;
            }

            // Retirer les entrées vides
            miner.storage.retain(|_, qty| *qty > 0);
        }
    }
}

fn auto_miner_production(
    time: Res<Time>,
    mut miner_query: Query<&mut AutoMiner>,
) {
    let mut rng = rand::thread_rng();

    for mut miner in miner_query.iter_mut() {
        let total: u32 = miner.storage.values().sum();
        if total >= MAX_MINER_STORAGE {
            continue;
        }

        miner.timer.tick(time.delta());

        if miner.timer.just_finished() {
            let roll: f32 = rng.gen();
            let drop_table = miner.deposit_type.drop_table();

            let mut produced = None;
            for (tile_type, threshold) in drop_table {
                if roll < *threshold {
                    produced = Some(tile_type.clone());
                    break;
                }
            }
            if let Some(tile_type) = produced {
                *miner.storage.entry(tile_type).or_insert(0) += 1;
            }
        }
    }
}
