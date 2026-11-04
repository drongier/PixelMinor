use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::economy::resources::{Inventory, PlayerStats, ShopOpen};
use crate::player::player::Player;
use crate::world::deposits::{Deposit, DepositType};
use crate::world::tiles::{GridPos, TileType, TILE_SIZE};

pub struct AutoMinerPlugin;

impl Plugin for AutoMinerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MinerInventory::default())
            .init_resource::<BKeyConsumed>()
            .add_systems(Update, (handle_b_key, auto_miner_production));
    }
}

#[derive(Resource, Default)]
pub struct MinerInventory {
    pub count: u32,
}

/// Mis à `true` par `handle_b_key` quand l'appui sur B de cette frame a déjà
/// déclenché une action (placer / collecter un mineur). Les systèmes de plus
/// basse priorité (ex: collecte en bout de tapis) doivent tourner
/// `.after(handle_b_key)` et ignorer l'appui si ce flag est levé.
#[derive(Resource, Default)]
pub struct BKeyConsumed(pub bool);

#[derive(Component)]
pub struct AutoMiner {
    pub deposit_type: DepositType,
    pub storage: HashMap<TileType, u32>,
    pub timer: Timer,
}

const MINER_PROXIMITY: f32 = TILE_SIZE * 2.5;
pub const MAX_MINER_STORAGE: u32 = 50;
const PRODUCTION_INTERVAL: f32 = 10.0;

/// Tire une ressource dans une table de seuils cumulés (`roll` dans [0, 1)).
pub fn roll_drop(table: &[(TileType, f32)], roll: f32) -> Option<TileType> {
    table
        .iter()
        .find(|(_, threshold)| roll < *threshold)
        .map(|(tile_type, _)| tile_type.clone())
}

/// Transfère au plus `capacity_left` ressources de `from` vers `to`.
/// Retire les entrées vides de `from`. Renvoie la quantité transférée.
pub fn transfer_storage(
    from: &mut HashMap<TileType, u32>,
    to: &mut HashMap<TileType, u32>,
    capacity_left: u32,
) -> u32 {
    let mut transferred = 0u32;
    for (tile_type, qty) in from.iter_mut() {
        if transferred >= capacity_left {
            break;
        }
        let can_take = (*qty).min(capacity_left - transferred);
        *to.entry(tile_type.clone()).or_insert(0) += can_take;
        *qty -= can_take;
        transferred += can_take;
    }
    from.retain(|_, qty| *qty > 0);
    transferred
}

#[allow(clippy::too_many_arguments, reason = "Bevy system: each param is a distinct resource/query")]
pub fn handle_b_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut consumed: ResMut<BKeyConsumed>,
    mut miner_inv: ResMut<MinerInventory>,
    mut inventory: ResMut<Inventory>,
    stats: Res<PlayerStats>,
    player_query: Query<&Transform, With<Player>>,
    deposit_query: Query<(&GridPos, &Deposit)>,
    mut miner_query: Query<(&GridPos, &Transform, &mut AutoMiner)>,
) {
    consumed.0 = false;
    if !keys.just_pressed(KeyCode::KeyB) || shop_open.0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();

    // Priorité 1 : placer un mineur si on en a et qu'un gisement libre est proche
    if miner_inv.count > 0 {
        let mut best: Option<(GridPos, DepositType, f32)> = None;

        for (grid_pos, deposit) in deposit_query.iter() {
            if !deposit.revealed {
                continue;
            }
            // Pas déjà de mineur sur cette tile
            if miner_query.iter().any(|(mp, _, _)| mp == grid_pos) {
                continue;
            }
            let dist = player_pos.distance(grid_pos.to_world());
            if dist <= MINER_PROXIMITY && best.as_ref().is_none_or(|b| dist < b.2) {
                best = Some((*grid_pos, deposit.deposit_type.clone(), dist));
            }
        }

        if let Some((grid_pos, deposit_type, _)) = best {
            miner_inv.count -= 1;
            consumed.0 = true;
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

    // Priorité 2 : collecter depuis le mineur le plus proche qui a du stock
    let best_miner = miner_query
        .iter_mut()
        .filter(|(_, _, miner)| !miner.storage.is_empty())
        .map(|(_, transform, miner)| (player_pos.distance(transform.translation.truncate()), miner))
        .filter(|(dist, _)| *dist <= MINER_PROXIMITY)
        .min_by(|a, b| a.0.total_cmp(&b.0));

    if let Some((_, mut miner)) = best_miner {
        consumed.0 = true;
        let capacity_left = stats.inventory_capacity.saturating_sub(inventory.total_count());
        transfer_storage(&mut miner.storage, &mut inventory.items, capacity_left);
    }
}

fn auto_miner_production(time: Res<Time>, mut miner_query: Query<&mut AutoMiner>) {
    let mut rng = rand::thread_rng();

    for mut miner in miner_query.iter_mut() {
        let total: u32 = miner.storage.values().sum();
        if total >= MAX_MINER_STORAGE {
            continue;
        }

        miner.timer.tick(time.delta());

        if miner.timer.just_finished() {
            let roll: f32 = rng.gen();
            if let Some(tile_type) = roll_drop(miner.deposit_type.drop_table(), roll) {
                *miner.storage.entry(tile_type).or_insert(0) += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_drop_picks_first_threshold_above_roll() {
        let table = DepositType::Ferrous.drop_table();
        assert_eq!(roll_drop(table, 0.0), Some(TileType::Iron));
        assert_eq!(roll_drop(table, 0.69), Some(TileType::Iron));
        assert_eq!(roll_drop(table, 0.70), Some(TileType::Coal));
        assert_eq!(roll_drop(table, 0.95), Some(TileType::Copper));
        assert_eq!(roll_drop(table, 0.999), Some(TileType::Silver));
    }

    #[test]
    fn roll_drop_out_of_range_is_none() {
        assert_eq!(roll_drop(DepositType::Gems.drop_table(), 1.0), None);
        assert_eq!(roll_drop(&[], 0.5), None);
    }

    #[test]
    fn every_drop_table_covers_full_range() {
        for dt in [DepositType::Ferrous, DepositType::Precious, DepositType::Gems, DepositType::Mythic] {
            assert!(roll_drop(dt.drop_table(), 0.9999).is_some());
        }
    }

    #[test]
    fn transfer_storage_respects_capacity() {
        let mut from = HashMap::from([(TileType::Iron, 5)]);
        let mut to = HashMap::new();
        assert_eq!(transfer_storage(&mut from, &mut to, 3), 3);
        assert_eq!(from.get(&TileType::Iron), Some(&2));
        assert_eq!(to.get(&TileType::Iron), Some(&3));

        assert_eq!(transfer_storage(&mut from, &mut to, 10), 2);
        assert!(from.is_empty());
        assert_eq!(to.get(&TileType::Iron), Some(&5));
    }
}
