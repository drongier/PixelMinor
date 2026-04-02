use bevy::prelude::*;
use std::collections::HashMap;

use crate::machines::auto_miner::AutoMiner;
use crate::player::player::Player;
use crate::economy::resources::{Inventory, PlayerStats, ShopOpen, Wallet};
use crate::world::tiles::{GridPos, TileType, TILE_SIZE};

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ConveyorState::default())
            .insert_resource(ConveyorBelts::default())
            .add_systems(Update, (
                toggle_tracing,
                trace_path,
                finalize_merge,
                conveyor_pull,
                conveyor_transport,
                transfer_between_belts,
                conveyor_deliver,
                collect_conveyor_end,
                update_conveyor_visuals,
            ));
    }
}

const CONVEYOR_COST: u64 = 10;
const TRANSPORT_INTERVAL: f32 = 5.0;
const CONVEYOR_COLOR: Color = Color::srgb(0.45, 0.45, 0.50);
const CONVEYOR_PREVIEW_COLOR: Color = Color::srgba(0.45, 0.45, 0.50, 0.5);

// Proximité pour considérer qu'on est "dans" la zone de vente (en tiles)
const SELL_ZONE_PROXIMITY: i32 = 3;

#[derive(Resource, Default)]
pub struct ConveyorState {
    pub tracing: bool,
    pub current_path: Vec<GridPos>,
    pub build_error: Option<&'static str>,
    last_grid: Option<GridPos>,
    preview_entities: Vec<Entity>,
    merge_target: Option<(u32, usize)>, // (belt_id, index) du tapis cible
}

#[derive(Resource, Default)]
pub struct ConveyorBelts {
    belts: Vec<Belt>,
    next_id: u32,
}

struct Belt {
    id: u32,
    path: Vec<GridPos>,
    slots: Vec<Option<TileType>>,
    timer: Timer,
    end_storage: HashMap<TileType, u32>,
    sells_at_end: bool,
    /// Si ce belt se connecte à un autre belt, (belt_id, index dans le path du belt cible)
    feeds_into: Option<(u32, usize)>,
}

#[derive(Component)]
struct ConveyorTile {
    belt_id: u32,
    index: usize,
}

fn is_near_sell_zone(pos: &GridPos) -> bool {
    let sell_grid = GridPos { x: 0, y: -2 };
    let dist = (pos.x - sell_grid.x).abs() + (pos.y - sell_grid.y).abs();
    dist <= SELL_ZONE_PROXIMITY
}

/// Vérifie si N'IMPORTE QUEL tile du path est adjacent à un mineur
fn path_connects_to_miner(path: &[GridPos], miner_positions: &[GridPos]) -> bool {
    for pos in path {
        let neighbors = pos.neighbors();
        for mp in miner_positions {
            if neighbors.contains(mp) || *mp == *pos {
                return true;
            }
        }
    }
    false
}

fn toggle_tracing(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut state: ResMut<ConveyorState>,
    mut belts: ResMut<ConveyorBelts>,
    mut wallet: ResMut<Wallet>,
    player_query: Query<&Transform, With<Player>>,
    miner_query: Query<&GridPos, With<AutoMiner>>,
) {
    if !keys.just_pressed(KeyCode::KeyT) || shop_open.0 {
        return;
    }

    // Reset build error
    state.build_error = None;

    if state.tracing {
        // Fin du traçage
        state.tracing = false;

        let path = std::mem::take(&mut state.current_path);
        let preview_entities = std::mem::take(&mut state.preview_entities);
        state.last_grid = None;

        if path.len() < 2 {
            let refund = path.len() as u64 * CONVEYOR_COST;
            wallet.money += refund;
            for entity in &preview_entities {
                commands.entity(*entity).despawn();
            }
            state.build_error = Some("Tapis trop court (min 2 tiles)");
            return;
        }

        // Vérifier qu'au moins un tile du path est adjacent à un mineur
        let miner_positions: Vec<GridPos> = miner_query.iter().copied().collect();
        let has_miner = path_connects_to_miner(&path, &miner_positions);

        if !has_miner {
            let refund = path.len() as u64 * CONVEYOR_COST;
            wallet.money += refund;
            for entity in &preview_entities {
                commands.entity(*entity).despawn();
            }
            state.build_error = Some("Aucun mineur adjacent au tapis !");
            return;
        }

        // Vérifier si la fin est dans la zone de vente
        let end = &path[path.len() - 1];
        let sells_at_end = is_near_sell_zone(end);

        let belt_id = belts.next_id;
        belts.next_id += 1;

        let slot_count = path.len();
        belts.belts.push(Belt {
            id: belt_id,
            path: path.clone(),
            slots: vec![None; slot_count],
            timer: Timer::from_seconds(TRANSPORT_INTERVAL, TimerMode::Repeating),
            end_storage: HashMap::new(),
            sells_at_end,
            feeds_into: None,
        });

        // Convertir les previews en tiles finales
        // preview_entities[i] correspond à path[i] (même index)
        for (i, entity) in preview_entities.iter().enumerate() {
            commands.entity(*entity).insert((
                ConveyorTile {
                    belt_id,
                    index: i,
                },
                path[i],
            ));
            // Mettre la couleur finale (opaque)
            commands.entity(*entity).insert(Sprite {
                color: CONVEYOR_COLOR,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                ..default()
            });
        }
    } else {
        // Début du traçage
        let player_pos = player_query.single().translation.truncate();
        let grid = GridPos {
            x: (player_pos.x / TILE_SIZE).round() as i32,
            y: (player_pos.y / TILE_SIZE).round() as i32,
        };

        state.tracing = true;
        state.current_path.clear();
        state.preview_entities.clear();
        state.last_grid = Some(grid);

        // Ajouter le premier tile AU path ET créer sa preview
        state.current_path.push(grid);
        let world = grid.to_world();
        let entity = commands.spawn((
            Sprite {
                color: CONVEYOR_PREVIEW_COLOR,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, -1.5),
        )).id();
        state.preview_entities.push(entity);
    }
}

fn trace_path(
    mut commands: Commands,
    mut state: ResMut<ConveyorState>,
    mut wallet: ResMut<Wallet>,
    player_query: Query<&Transform, With<Player>>,
    conveyor_tile_query: Query<(&GridPos, &ConveyorTile)>,
) {
    if !state.tracing {
        return;
    }

    let player_pos = player_query.single().translation.truncate();
    let grid = GridPos {
        x: (player_pos.x / TILE_SIZE).round() as i32,
        y: (player_pos.y / TILE_SIZE).round() as i32,
    };

    if let Some(last) = state.last_grid {
        if grid == last {
            return;
        }
    }

    if state.current_path.contains(&grid) {
        return;
    }

    // Vérifier si on marche sur un tapis existant → fusion automatique
    let existing = conveyor_tile_query.iter().find(|(pos, _)| **pos == grid);
    if let Some((_, conv_tile)) = existing {
        // Marquer la fusion : on ajoute le grid au path mais on arrête le traçage
        state.current_path.push(grid);
        state.last_grid = Some(grid);
        // Stocker les infos de fusion dans build_error temporairement
        // On va gérer ça proprement via un flag
        state.tracing = false;
        state.merge_target = Some((conv_tile.belt_id, conv_tile.index));
        return;
    }

    if wallet.money < CONVEYOR_COST {
        state.tracing = false;
        state.build_error = Some("Plus assez d'argent !");
        return;
    }

    wallet.money -= CONVEYOR_COST;
    state.current_path.push(grid);
    state.last_grid = Some(grid);

    let world = grid.to_world();
    let entity = commands.spawn((
        Sprite {
            color: CONVEYOR_PREVIEW_COLOR,
            custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
            ..default()
        },
        Transform::from_xyz(world.x, world.y, -1.5),
    )).id();
    state.preview_entities.push(entity);
}

/// Quand trace_path détecte une fusion, ce système finalise le nouveau belt
fn finalize_merge(
    mut commands: Commands,
    mut state: ResMut<ConveyorState>,
    mut belts: ResMut<ConveyorBelts>,
    mut wallet: ResMut<Wallet>,
    miner_query: Query<&GridPos, With<AutoMiner>>,
) {
    let merge_target = match state.merge_target.take() {
        Some(t) => t,
        None => return,
    };

    let path = std::mem::take(&mut state.current_path);
    let preview_entities = std::mem::take(&mut state.preview_entities);
    state.last_grid = None;

    if path.len() < 2 {
        for entity in &preview_entities {
            commands.entity(*entity).despawn();
        }
        state.build_error = Some("Tapis trop court (min 2 tiles)");
        return;
    }

    // Vérifier la connexion à un mineur
    let miner_positions: Vec<GridPos> = miner_query.iter().copied().collect();
    let has_miner = path_connects_to_miner(&path, &miner_positions);

    if !has_miner {
        let refund = (path.len() - 1) as u64 * CONVEYOR_COST;
        wallet.money += refund;
        for entity in &preview_entities {
            commands.entity(*entity).despawn();
        }
        state.build_error = Some("Aucun mineur adjacent au tapis !");
        return;
    }

    let belt_id = belts.next_id;
    belts.next_id += 1;

    let slot_count = path.len() - 1; // Le dernier tile est celui du tapis existant, pas un slot
    belts.belts.push(Belt {
        id: belt_id,
        path: path[..path.len() - 1].to_vec(), // Path sans le tile de fusion
        slots: vec![None; slot_count],
        timer: Timer::from_seconds(TRANSPORT_INTERVAL, TimerMode::Repeating),
        end_storage: HashMap::new(),
        sells_at_end: false,
        feeds_into: Some(merge_target),
    });

    // Convertir les previews en tiles finales (sauf la dernière qui est le tile existant)
    for (i, entity) in preview_entities.iter().enumerate() {
        commands.entity(*entity).insert((
            ConveyorTile {
                belt_id,
                index: i,
            },
            path[i],
        ));
        commands.entity(*entity).insert(Sprite {
            color: CONVEYOR_COLOR,
            custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
            ..default()
        });
    }
}

/// Transfère les ressources d'un belt qui feeds_into un autre
fn transfer_between_belts(
    mut belts: ResMut<ConveyorBelts>,
) {
    // Collecter les transferts à faire
    let mut transfers: Vec<(u32, usize, TileType)> = Vec::new();

    for belt in belts.belts.iter() {
        if let Some((target_belt_id, target_index)) = belt.feeds_into {
            if !belt.end_storage.is_empty() {
                // Prendre le premier item du end_storage
                if let Some((tile_type, _)) = belt.end_storage.iter().find(|(_, qty)| **qty > 0) {
                    transfers.push((target_belt_id, target_index, tile_type.clone()));
                }
            }
        }
    }

    // Appliquer les transferts
    for (target_belt_id, target_index, tile_type) in transfers {
        // Retirer du source
        let source = belts.belts.iter_mut().find(|b| {
            b.feeds_into == Some((target_belt_id, target_index))
        });
        if let Some(source) = source {
            if let Some(qty) = source.end_storage.get_mut(&tile_type) {
                *qty -= 1;
                if *qty == 0 {
                    source.end_storage.remove(&tile_type);
                }
            }
        }

        // Injecter dans le target
        let target = belts.belts.iter_mut().find(|b| b.id == target_belt_id);
        if let Some(target) = target {
            if target_index < target.slots.len() && target.slots[target_index].is_none() {
                target.slots[target_index] = Some(tile_type);
            }
        }
    }
}

fn conveyor_pull(
    mut belts: ResMut<ConveyorBelts>,
    mut miner_query: Query<(&GridPos, &mut AutoMiner)>,
) {
    for belt in belts.belts.iter_mut() {
        if belt.slots[0].is_some() {
            continue;
        }

        let start = &belt.path[0];
        let start_neighbors = start.neighbors();

        for (miner_pos, mut miner) in miner_query.iter_mut() {
            if !start_neighbors.contains(miner_pos) && *miner_pos != *start {
                continue;
            }

            let mut taken = None;
            for (tile_type, qty) in miner.storage.iter_mut() {
                if *qty > 0 {
                    *qty -= 1;
                    taken = Some(tile_type.clone());
                    break;
                }
            }

            if let Some(tile_type) = taken {
                miner.storage.retain(|_, qty| *qty > 0);
                belt.slots[0] = Some(tile_type);
                break;
            }
        }
    }
}

fn conveyor_transport(
    time: Res<Time>,
    mut belts: ResMut<ConveyorBelts>,
) {
    for belt in belts.belts.iter_mut() {
        belt.timer.tick(time.delta());

        if !belt.timer.just_finished() {
            continue;
        }

        let len = belt.slots.len();
        if let Some(tile_type) = belt.slots[len - 1].take() {
            *belt.end_storage.entry(tile_type).or_insert(0) += 1;
        }

        for i in (1..len).rev() {
            if belt.slots[i].is_none() {
                belt.slots[i] = belt.slots[i - 1].take();
            }
        }
    }
}

fn update_conveyor_visuals(
    belts: Res<ConveyorBelts>,
    mut tile_query: Query<(&ConveyorTile, &mut Sprite)>,
) {
    for (conv_tile, mut sprite) in tile_query.iter_mut() {
        let Some(belt) = belts.belts.iter().find(|b| b.id == conv_tile.belt_id) else {
            continue;
        };

        if conv_tile.index >= belt.slots.len() {
            continue;
        }

        sprite.color = match &belt.slots[conv_tile.index] {
            Some(tile_type) => tile_type.color(),
            None => CONVEYOR_COLOR,
        };
    }
}

fn conveyor_deliver(
    mut belts: ResMut<ConveyorBelts>,
    mut wallet: ResMut<Wallet>,
) {
    for belt in belts.belts.iter_mut() {
        if !belt.sells_at_end || belt.end_storage.is_empty() {
            continue;
        }

        let total: u64 = belt.end_storage
            .iter()
            .map(|(t, qty)| t.value() * (*qty as u64))
            .sum();

        wallet.money += total;
        belt.end_storage.clear();
    }
}

fn collect_conveyor_end(
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut belts: ResMut<ConveyorBelts>,
    mut inventory: ResMut<Inventory>,
    stats: Res<PlayerStats>,
    player_query: Query<&Transform, With<Player>>,
) {
    if !keys.just_pressed(KeyCode::KeyB) || shop_open.0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();
    let player_grid = GridPos {
        x: (player_pos.x / TILE_SIZE).round() as i32,
        y: (player_pos.y / TILE_SIZE).round() as i32,
    };

    for belt in belts.belts.iter_mut() {
        if belt.sells_at_end || belt.end_storage.is_empty() {
            continue;
        }

        let end = belt.path[belt.path.len() - 1];
        let dist = (player_grid.x - end.x).abs() + (player_grid.y - end.y).abs();

        if dist > 2 {
            continue;
        }

        let capacity_left = stats.inventory_capacity.saturating_sub(inventory.total_count());
        let mut transferred = 0u32;

        for (tile_type, qty) in belt.end_storage.iter_mut() {
            if transferred >= capacity_left {
                break;
            }
            let can_take = (*qty).min(capacity_left - transferred);
            *inventory.items.entry(tile_type.clone()).or_insert(0) += can_take;
            *qty -= can_take;
            transferred += can_take;
        }

        belt.end_storage.retain(|_, qty| *qty > 0);
    }
}
