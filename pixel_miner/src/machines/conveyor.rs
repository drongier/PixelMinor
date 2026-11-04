use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::resources::{Inventory, PlayerStats, ShopOpen, Wallet};
use crate::machines::auto_miner::{handle_b_key, transfer_storage, AutoMiner, BKeyConsumed};
use crate::player::player::Player;
use crate::world::tiles::{total_value, GridPos, TileType, SELL_ZONE_POS, TILE_SIZE};

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ConveyorState::default())
            .insert_resource(ConveyorBelts::default())
            .init_resource::<BKeyConsumed>()
            .add_systems(Update, (
                // Chaînés : une fusion détectée par trace_path est finalisée dans la
                // même frame, avant qu'un nouvel appui sur T ne puisse la perdre.
                (toggle_tracing, trace_path, finalize_merge).chain(),
                conveyor_pull,
                conveyor_transport,
                transfer_between_belts.after(conveyor_transport),
                conveyor_deliver.after(transfer_between_belts),
                // B : placer/collecter un mineur est prioritaire sur le bout de tapis
                collect_conveyor_end.after(handle_b_key),
                update_conveyor_visuals,
            ));
    }
}

pub const CONVEYOR_COST: u64 = 10;
const TRANSPORT_INTERVAL: f32 = 5.0;
const CONVEYOR_COLOR: Color = Color::srgb(0.45, 0.45, 0.50);
const CONVEYOR_PREVIEW_COLOR: Color = Color::srgba(0.45, 0.45, 0.50, 0.5);

// Proximité pour considérer qu'on est "dans" la zone de vente (en tiles)
const SELL_ZONE_PROXIMITY: i32 = 3;
// Distance max (en tiles) pour collecter en bout de tapis
const BELT_END_PROXIMITY: i32 = 2;

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

/// Montant payé pour un tracé de `path_len` tiles.
///
/// Chaque tile posée est payée (y compris la première, au démarrage du tracé),
/// sauf lors d'une fusion : la dernière tile du path est alors celle du tapis
/// existant, qui est gratuite. Tout remboursement doit utiliser cette fonction
/// pour être strictement égal à ce qui a été débité.
pub fn path_cost(path_len: usize, merging: bool) -> u64 {
    let paid_tiles = if merging { path_len.saturating_sub(1) } else { path_len };
    paid_tiles as u64 * CONVEYOR_COST
}

fn is_near_sell_zone(pos: &GridPos) -> bool {
    // La zone de vente est centrée à mi-chemin entre deux cases (y = -2.5 tiles) :
    // on prend la case côté base (y = -2) pour conserver le comportement historique.
    let sell_grid = GridPos::from_world(SELL_ZONE_POS + Vec2::new(0.0, TILE_SIZE / 2.0));
    pos.manhattan(&sell_grid) <= SELL_ZONE_PROXIMITY
}

/// Vérifie si N'IMPORTE QUEL tile du path est sur ou adjacent à un mineur
fn path_connects_to_miner(path: &[GridPos], miner_positions: &[GridPos]) -> bool {
    path.iter()
        .any(|pos| miner_positions.iter().any(|mp| pos.manhattan(mp) <= 1))
}

fn spawn_preview(commands: &mut Commands, grid: GridPos) -> Entity {
    let world = grid.to_world();
    commands
        .spawn((
            Sprite {
                color: CONVEYOR_PREVIEW_COLOR,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, -1.5),
        ))
        .id()
}

/// Annule un tracé : supprime les previews et rembourse exactement ce qui a été payé.
fn cancel_path(
    commands: &mut Commands,
    wallet: &mut Wallet,
    path_len: usize,
    merging: bool,
    preview_entities: &[Entity],
) {
    wallet.money += path_cost(path_len, merging);
    for entity in preview_entities {
        commands.entity(*entity).despawn();
    }
}

/// Transforme les previews en tiles de tapis définitives.
/// `preview_entities[i]` correspond à `path[i]`.
fn commit_previews(commands: &mut Commands, belt_id: u32, path: &[GridPos], preview_entities: &[Entity]) {
    for (i, entity) in preview_entities.iter().enumerate() {
        commands.entity(*entity).insert((
            ConveyorTile { belt_id, index: i },
            path[i],
            Sprite {
                color: CONVEYOR_COLOR,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                ..default()
            },
        ));
    }
}

#[allow(clippy::too_many_arguments, reason = "Bevy system: each param is a distinct resource/query")]
fn toggle_tracing(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut state: ResMut<ConveyorState>,
    mut belts: ResMut<ConveyorBelts>,
    mut wallet: ResMut<Wallet>,
    player_query: Query<&Transform, With<Player>>,
    miner_query: Query<&GridPos, With<AutoMiner>>,
    conveyor_tile_query: Query<&GridPos, With<ConveyorTile>>,
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
            cancel_path(&mut commands, &mut wallet, path.len(), false, &preview_entities);
            state.build_error = Some("Tapis trop court (min 2 tiles)");
            return;
        }

        // Vérifier qu'au moins un tile du path est adjacent à un mineur
        let miner_positions: Vec<GridPos> = miner_query.iter().copied().collect();
        if !path_connects_to_miner(&path, &miner_positions) {
            cancel_path(&mut commands, &mut wallet, path.len(), false, &preview_entities);
            state.build_error = Some("Aucun mineur adjacent au tapis !");
            return;
        }

        let belt_id = belts.next_id;
        belts.next_id += 1;

        commit_previews(&mut commands, belt_id, &path, &preview_entities);
        belts.belts.push(Belt {
            id: belt_id,
            sells_at_end: is_near_sell_zone(&path[path.len() - 1]),
            slots: vec![None; path.len()],
            path,
            timer: Timer::from_seconds(TRANSPORT_INTERVAL, TimerMode::Repeating),
            end_storage: HashMap::new(),
            feeds_into: None,
        });
    } else {
        // Début du traçage
        let player_pos = player_query.single().translation.truncate();
        let grid = GridPos::from_world(player_pos);

        if conveyor_tile_query.iter().any(|pos| *pos == grid) {
            state.build_error = Some("Il y a déjà un tapis ici !");
            return;
        }

        // La première tile est payée dès le début du tracé
        if wallet.money < CONVEYOR_COST {
            state.build_error = Some("Pas assez d'argent !");
            return;
        }
        wallet.money -= CONVEYOR_COST;

        state.tracing = true;
        state.current_path.clear();
        state.current_path.push(grid);
        state.last_grid = Some(grid);
        let entity = spawn_preview(&mut commands, grid);
        state.preview_entities.clear();
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
    let grid = GridPos::from_world(player_pos);

    if state.last_grid == Some(grid) {
        return;
    }

    // Le tapis en cours de tracé n'est pas fusionnable avec lui-même : ses tiles
    // sont encore des previews (sans ConveyorTile) et sont ignorées ici.
    if state.current_path.contains(&grid) {
        return;
    }

    // Marcher sur un tapis existant → fusion automatique (tile gratuite)
    if let Some((_, conv_tile)) = conveyor_tile_query.iter().find(|(pos, _)| **pos == grid) {
        state.current_path.push(grid);
        state.last_grid = Some(grid);
        state.tracing = false;
        state.merge_target = Some((conv_tile.belt_id, conv_tile.index));
        return;
    }

    // Plus d'argent : on n'étend plus le tracé, le joueur peut valider avec T.
    if wallet.money < CONVEYOR_COST {
        state.build_error = Some("Plus assez d'argent !");
        return;
    }

    wallet.money -= CONVEYOR_COST;
    state.current_path.push(grid);
    state.last_grid = Some(grid);
    let entity = spawn_preview(&mut commands, grid);
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
    let Some(merge_target) = state.merge_target.take() else {
        return;
    };

    let path = std::mem::take(&mut state.current_path);
    let preview_entities = std::mem::take(&mut state.preview_entities);
    state.last_grid = None;

    if path.len() < 2 {
        cancel_path(&mut commands, &mut wallet, path.len(), true, &preview_entities);
        state.build_error = Some("Tapis trop court (min 2 tiles)");
        return;
    }

    // Vérifier la connexion à un mineur
    let miner_positions: Vec<GridPos> = miner_query.iter().copied().collect();
    if !path_connects_to_miner(&path, &miner_positions) {
        cancel_path(&mut commands, &mut wallet, path.len(), true, &preview_entities);
        state.build_error = Some("Aucun mineur adjacent au tapis !");
        return;
    }

    let belt_id = belts.next_id;
    belts.next_id += 1;

    // Le dernier tile est celui du tapis existant : ni slot ni preview
    let own_path = path[..path.len() - 1].to_vec();
    commit_previews(&mut commands, belt_id, &own_path, &preview_entities);
    belts.belts.push(Belt {
        id: belt_id,
        slots: vec![None; own_path.len()],
        path: own_path,
        timer: Timer::from_seconds(TRANSPORT_INTERVAL, TimerMode::Repeating),
        end_storage: HashMap::new(),
        sells_at_end: false,
        feeds_into: Some(merge_target),
    });
}

/// Transfère les ressources d'un belt qui feeds_into un autre
fn transfer_between_belts(mut belts: ResMut<ConveyorBelts>) {
    // Transferts à faire : (source_belt_id, target_belt_id, target_index, tile_type)
    let transfers: Vec<(u32, u32, usize, TileType)> = belts
        .belts
        .iter()
        .filter_map(|belt| {
            let (target_belt_id, target_index) = belt.feeds_into?;
            let (tile_type, _) = belt.end_storage.iter().find(|(_, qty)| **qty > 0)?;
            Some((belt.id, target_belt_id, target_index, tile_type.clone()))
        })
        .collect();

    for (source_id, target_belt_id, target_index, tile_type) in transfers {
        // Slot libre sur le tapis cible : jonction d'abord, puis les suivants
        let injected = belts
            .belts
            .iter_mut()
            .find(|b| b.id == target_belt_id)
            .and_then(|target| target.slots.iter_mut().skip(target_index).find(|s| s.is_none()))
            .map(|slot| *slot = Some(tile_type.clone()))
            .is_some();

        // Retirer du source seulement si l'injection a réussi
        if !injected {
            continue;
        }
        if let Some(source) = belts.belts.iter_mut().find(|b| b.id == source_id) {
            if let Some(qty) = source.end_storage.get_mut(&tile_type) {
                *qty -= 1;
                if *qty == 0 {
                    source.end_storage.remove(&tile_type);
                }
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

        let start = belt.path[0];

        for (miner_pos, mut miner) in miner_query.iter_mut() {
            if start.manhattan(miner_pos) > 1 {
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

fn conveyor_transport(time: Res<Time>, mut belts: ResMut<ConveyorBelts>) {
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
        let Some(slot) = belt.slots.get(conv_tile.index) else {
            continue;
        };

        sprite.color = match slot {
            Some(tile_type) => tile_type.color(),
            None => CONVEYOR_COLOR,
        };
    }
}

fn conveyor_deliver(mut belts: ResMut<ConveyorBelts>, mut wallet: ResMut<Wallet>) {
    for belt in belts.belts.iter_mut() {
        if !belt.sells_at_end || belt.end_storage.is_empty() {
            continue;
        }

        wallet.money += total_value(&belt.end_storage);
        belt.end_storage.clear();
    }
}

fn collect_conveyor_end(
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    consumed: Res<BKeyConsumed>,
    mut belts: ResMut<ConveyorBelts>,
    mut inventory: ResMut<Inventory>,
    stats: Res<PlayerStats>,
    player_query: Query<&Transform, With<Player>>,
) {
    // Appui déjà utilisé par le mineur (placement / collecte) → rien ici
    if !keys.just_pressed(KeyCode::KeyB) || shop_open.0 || consumed.0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();
    let player_grid = GridPos::from_world(player_pos);

    for belt in belts.belts.iter_mut() {
        if belt.sells_at_end || belt.end_storage.is_empty() {
            continue;
        }

        let end = belt.path[belt.path.len() - 1];
        if player_grid.manhattan(&end) > BELT_END_PROXIMITY {
            continue;
        }

        let capacity_left = stats.inventory_capacity.saturating_sub(inventory.total_count());
        transfer_storage(&mut belt.end_storage, &mut inventory.items, capacity_left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simule le débit réel : 1 tile payée au démarrage, puis 1 par tile tracée,
    /// et 0 pour la tile de fusion.
    fn simulate_paid(placed_tiles: usize) -> u64 {
        let first = CONVEYOR_COST; // toggle_tracing
        let traced = (placed_tiles - 1) as u64 * CONVEYOR_COST; // trace_path
        first + traced
    }

    #[test]
    fn start_only_refund_equals_paid() {
        // T T : un seul tile, trop court → remboursé intégralement
        assert_eq!(path_cost(1, false), simulate_paid(1));
        assert_eq!(path_cost(1, false), CONVEYOR_COST);
    }

    #[test]
    fn normal_belt_cost_equals_paid() {
        for len in 2..10 {
            assert_eq!(path_cost(len, false), simulate_paid(len));
        }
    }

    #[test]
    fn failed_belt_refund_equals_paid() {
        // Tapis sans mineur adjacent : refund == paid, donc bilan net nul
        let len = 5;
        let paid = simulate_paid(len);
        let refund = path_cost(len, false);
        assert_eq!(paid, refund);
    }

    #[test]
    fn merge_belt_refund_equals_paid() {
        // path = tiles posées + tile du tapis existant (gratuite)
        for placed in 1..10 {
            let path_len = placed + 1;
            assert_eq!(path_cost(path_len, true), simulate_paid(placed));
        }
    }

    #[test]
    fn repeated_start_stop_does_not_print_money() {
        let mut money: u64 = 500;
        for _ in 0..100 {
            money -= CONVEYOR_COST; // démarrage
            money += path_cost(1, false); // arrêt immédiat
        }
        assert_eq!(money, 500);
    }

    #[test]
    fn path_connects_to_adjacent_miner_only() {
        let path = [GridPos { x: 0, y: 0 }, GridPos { x: 1, y: 0 }];
        assert!(path_connects_to_miner(&path, &[GridPos { x: 2, y: 0 }]));
        assert!(path_connects_to_miner(&path, &[GridPos { x: 0, y: 0 }]));
        assert!(!path_connects_to_miner(&path, &[GridPos { x: 2, y: 1 }]));
        assert!(!path_connects_to_miner(&path, &[]));
    }
}
