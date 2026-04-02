use bevy::prelude::*;

use crate::machines::auto_miner::{AutoMiner, MinerInventory};
use crate::machines::conveyor::ConveyorState;
use crate::world::deposits::Deposit;
use crate::player::player::Player;
use crate::economy::resources::{Inventory, PlayerStats, Wallet};
use crate::machines::scout_bot::BotInventory;
use crate::world::tiles::{GridPos, TileType};

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

fn update_hud(
    inventory: Res<Inventory>,
    wallet: Res<Wallet>,
    stats: Res<PlayerStats>,
    mut inv_query: Query<&mut Text, (With<HudInventoryText>, Without<HudMoneyText>, Without<HudStatsText>)>,
    mut money_query: Query<&mut Text, (With<HudMoneyText>, Without<HudInventoryText>, Without<HudStatsText>)>,
) {
    if inventory.is_changed() {
        let mut text = inv_query.single_mut();
        if inventory.items.is_empty() {
            *text = Text::new(format!("Inventaire (0/{}) : vide", stats.inventory_capacity));
        } else {
            let parts: Vec<String> = TileType::all_ordered()
                .iter()
                .filter_map(|t| {
                    inventory.items.get(t).map(|qty| format!("{} x{}", t.label(), qty))
                })
                .collect();
            *text = Text::new(format!(
                "Inventaire ({}/{}) : {}",
                inventory.total_count(),
                stats.inventory_capacity,
                parts.join("   ")
            ));
        }
    }

    if wallet.is_changed() {
        let mut text = money_query.single_mut();
        *text = Text::new(format!("{} c", wallet.money));
    }
}

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
    mut text_query: Query<&mut Text, (With<HudStatsText>, Without<HudMoneyText>, Without<HudInventoryText>)>,
) {
    let pos = player_query.single().translation.truncate();
    let grid_x = (pos.x / 16.0).round() as i32;
    let grid_y = (pos.y / 16.0).round() as i32;
    let dist = grid_x.abs() + grid_y.abs();
    let player_grid = GridPos { x: grid_x, y: grid_y };

    let zone = match dist {
        0..=2 => "Base",
        3..=20 => "Zone 1",
        21..=45 => "Zone 2",
        46..=75 => "Zone 3",
        76..=110 => "Zone 4",
        _ => "Zone 5",
    };

    let mut info = format!(
        "-- Stats --\n\
         Pos: ({}, {})\n\
         Zone: {} (dist {})\n\
         Puissance: {}\n\
         Vitesse: {}\n\
         Sac: {}/{}\n\
         Argent: {} c\n\
         Mineurs: {}\n\
         Bots: {}",
        grid_x, grid_y,
        zone, dist,
        stats.mining_power,
        stats.walk_speed,
        inventory.total_count(), stats.inventory_capacity,
        wallet.money,
        miner_inv.count,
        bot_inv.count,
    );

    if conveyor_state.tracing {
        let tile_count = conveyor_state.current_path.len();
        let cost = tile_count as u64 * 10;
        info.push_str(&format!(
            "\n\n>> MODE TAPIS <<\nTiles: {} (cout: {} c)\n[T] Terminer",
            tile_count, cost
        ));
    } else if let Some(error) = conveyor_state.build_error {
        info.push_str(&format!("\n\n!! {} !!", error));
    }

    // Afficher les infos du gisement si le joueur est dessus
    for (grid_pos, deposit) in deposit_query.iter() {
        if *grid_pos == player_grid && deposit.revealed {
            info.push_str(&format!("\n\n-- Gisement --\n{}\n", deposit.deposit_type.label()));
            info.push_str("Drop:\n");
            let drop_table = deposit.deposit_type.drop_table();
            let mut prev = 0.0_f32;
            for (tile_type, threshold) in drop_table {
                let pct = (threshold - prev) * 100.0;
                info.push_str(&format!("  {} : {:.0}%\n", tile_type.label(), pct));
                prev = *threshold;
            }

            // Vérifier si un mineur est posé dessus
            let has_miner = miner_query.iter().any(|(mp, _)| *mp == *grid_pos);
            if has_miner {
                if let Some((_, miner)) = miner_query.iter().find(|(mp, _)| **mp == *grid_pos) {
                    let total: u32 = miner.storage.values().sum();
                    info.push_str(&format!("Mineur: stock {}/50", total));
                }
            } else {
                info.push_str("[B] Poser un mineur");
            }
            break;
        }
    }

    let mut text = text_query.single_mut();
    *text = Text::new(info);
}
