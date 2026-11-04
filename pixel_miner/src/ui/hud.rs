use bevy::prelude::*;

use crate::machines::auto_miner::{AutoMiner, MinerInventory, MAX_MINER_STORAGE};
use crate::machines::conveyor::{path_cost, ConveyorState};
use crate::world::deposits::Deposit;
use crate::player::player::Player;
use crate::economy::resources::{Inventory, PlayerStats, Wallet};
use crate::machines::scout_bot::BotInventory;
use crate::world::tiles::{GridPos, TileType, Zone};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, (update_hud, update_stats_panel));
    }
}

#[derive(Component)]
struct HudInventoryText;

#[derive(Component)]
struct HudMoneyText;

#[derive(Component)]
struct HudStatsText;

const INVENTORY_COLOR: Color = Color::WHITE;
const INVENTORY_FULL_COLOR: Color = Color::srgb(1.0, 0.25, 0.25);

type InventoryTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<HudInventoryText>, Without<HudMoneyText>, Without<HudStatsText>),
>;
type MoneyTextQuery<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<HudMoneyText>, Without<HudInventoryText>, Without<HudStatsText>)>;
type StatsTextQuery<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<HudStatsText>, Without<HudMoneyText>, Without<HudInventoryText>)>;

fn setup_hud(mut commands: Commands) {
    // Barre du haut (argent)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudMoneyText,
                Text::new("0 c"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
            ));
        });

    // Panneau stats (haut gauche, sous la barre d'argent)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudStatsText,
                Text::new(""),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });

    // Barre du bas (inventaire)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudInventoryText,
                Text::new("Inventaire : vide"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

/// Texte de la barre d'inventaire (pur, testable).
fn inventory_bar_text(inventory: &Inventory, stats: &PlayerStats) -> String {
    let count = inventory.total_count();
    let cap = stats.inventory_capacity;
    let content = if inventory.items.is_empty() {
        "vide".to_string()
    } else {
        TileType::all_ordered()
            .iter()
            .filter_map(|t| inventory.items.get(t).map(|qty| format!("{} x{}", t.label(), qty)))
            .collect::<Vec<_>>()
            .join("   ")
    };
    let mut text = format!("Inventaire ({}/{}) : {}", count, cap, content);
    if inventory.is_full(stats) {
        text.push_str("   !! SAC PLEIN — retourne vendre ! !!");
    }
    text
}

fn update_hud(
    inventory: Res<Inventory>,
    wallet: Res<Wallet>,
    stats: Res<PlayerStats>,
    mut inv_query: InventoryTextQuery,
    mut money_query: MoneyTextQuery,
) {
    // Redessine aussi quand les stats changent (ex: amélioration du sac)
    if inventory.is_changed() || stats.is_changed() {
        let (mut text, mut color) = inv_query.single_mut();
        *text = Text::new(inventory_bar_text(&inventory, &stats));
        let wanted = if inventory.is_full(&stats) { INVENTORY_FULL_COLOR } else { INVENTORY_COLOR };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }

    if wallet.is_changed() {
        let mut text = money_query.single_mut();
        *text = Text::new(format!("{} c", wallet.money));
    }
}

#[allow(clippy::too_many_arguments)] // système Bevy : une ressource/query par paramètre
fn update_stats_panel(
    stats: Res<PlayerStats>,
    inventory: Res<Inventory>,
    wallet: Res<Wallet>,
    miner_inv: Res<MinerInventory>,
    bot_inv: Res<BotInventory>,
    conveyor_state: Res<ConveyorState>,
    player_query: Query<&Transform, With<Player>>,
    deposit_query: Query<(&GridPos, &Deposit), Without<AutoMiner>>,
    miner_query: Query<(&GridPos, &AutoMiner)>,
    mut text_query: StatsTextQuery,
) {
    let pos = player_query.single().translation.truncate();
    let player_grid = GridPos::from_world(pos);
    let (grid_x, grid_y) = (player_grid.x, player_grid.y);
    let dist = player_grid.distance_from_center();
    let zone = Zone::from_distance(dist).label();

    let full = if inventory.is_full(&stats) { " PLEIN" } else { "" };
    let mut info = format!(
        "-- Stats --\n\
         Pos: ({}, {})\n\
         Zone: {} (dist {})\n\
         Puissance: {}\n\
         Vitesse: {}\n\
         Sac: {}/{}{}\n\
         Argent: {} c\n\
         Mineurs: {}\n\
         Bots: {}",
        grid_x, grid_y,
        zone, dist,
        stats.mining_power,
        stats.walk_speed,
        inventory.total_count(), stats.inventory_capacity, full,
        wallet.money,
        miner_inv.count,
        bot_inv.count,
    );

    if conveyor_state.tracing {
        let tile_count = conveyor_state.current_path.len();
        // Pendant le tracé il n'y a pas de fusion (elle termine le tracé)
        let cost = path_cost(tile_count, false);
        info.push_str(&format!(
            "\n\n>> MODE TAPIS <<\nTiles: {} (cout: {} c)\n[T] Terminer",
            tile_count, cost
        ));
    } else if let Some(error) = conveyor_state.build_error {
        info.push_str(&format!("\n\n!! {} !!", error));
    }

    // Afficher les infos du gisement si le joueur est dessus
    if let Some((grid_pos, deposit)) = deposit_query
        .iter()
        .find(|(gp, d)| **gp == player_grid && d.revealed)
    {
        info.push_str(&format!("\n\n-- Gisement --\n{}\n", deposit.deposit_type.label()));
        info.push_str("Drop:\n");
        let mut prev = 0.0_f32;
        for (tile_type, threshold) in deposit.deposit_type.drop_table() {
            let pct = (threshold - prev) * 100.0;
            info.push_str(&format!("  {} : {:.0}%\n", tile_type.label(), pct));
            prev = *threshold;
        }

        match miner_query.iter().find(|(mp, _)| *mp == grid_pos) {
            Some((_, miner)) => {
                let total: u32 = miner.storage.values().sum();
                info.push_str(&format!("Mineur: stock {}/{}", total, MAX_MINER_STORAGE));
            }
            None => info.push_str("[B] Poser un mineur"),
        }
    }

    // N'écrit le texte (et ne déclenche le re-layout) que si le contenu a changé
    let mut text = text_query.single_mut();
    if text.0 != info {
        text.0 = info;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_bar_shows_full_warning_only_when_full() {
        let stats = PlayerStats::default();
        let mut inv = Inventory::default();
        assert!(!inventory_bar_text(&inv, &stats).contains("SAC PLEIN"));
        inv.items.insert(TileType::Stone, stats.inventory_capacity - 1);
        assert!(!inventory_bar_text(&inv, &stats).contains("SAC PLEIN"));
        inv.items.insert(TileType::Stone, stats.inventory_capacity);
        let text = inventory_bar_text(&inv, &stats);
        assert!(text.contains("SAC PLEIN"));
        assert!(text.contains(&format!("{}/{}", stats.inventory_capacity, stats.inventory_capacity)));
    }
}
