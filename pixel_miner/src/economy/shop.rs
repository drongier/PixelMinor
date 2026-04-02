use bevy::prelude::*;

use crate::machines::auto_miner::MinerInventory;
use crate::player::player::Player;
use crate::economy::resources::{PlayerStats, ShopOpen, Wallet};
use crate::machines::scout_bot::BotInventory;
use crate::world::tiles::TILE_SIZE;

const SHOP_ZONE_SIZE: f32 = TILE_SIZE * 2.0;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ShopState::default())
            .add_systems(Startup, spawn_shop_zone)
            .add_systems(Update, (shop_toggle, shop_buy));
    }
}

// --- Upgrade definitions ---

struct UpgradeTier {
    name: &'static str,
    value: f32,
    cost: u64,
}

const PICKAXE_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Pioche en bois",   value: 1.0,  cost: 0 },
    UpgradeTier { name: "Pioche en pierre",  value: 2.0,  cost: 50 },
    UpgradeTier { name: "Pioche en fer",     value: 4.0,  cost: 200 },
    UpgradeTier { name: "Pioche en or",      value: 7.0,  cost: 1000 },
    UpgradeTier { name: "Pioche diamant",    value: 12.0, cost: 5000 },
];

const BAG_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Poches",    value: 10.0,  cost: 0 },
    UpgradeTier { name: "Petit sac", value: 20.0,  cost: 100 },
    UpgradeTier { name: "Sac a dos", value: 40.0,  cost: 500 },
    UpgradeTier { name: "Chariot",   value: 80.0,  cost: 2000 },
    UpgradeTier { name: "Wagon",     value: 150.0, cost: 10000 },
];

const SPEED_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Normal",      value: 1.0, cost: 0 },
    UpgradeTier { name: "Rapide",      value: 1.5, cost: 300 },
    UpgradeTier { name: "Tres rapide", value: 2.0, cost: 1500 },
    UpgradeTier { name: "Frenetique",  value: 3.0, cost: 8000 },
];

const AUTO_MINER_COST: u64 = 100;
const SCOUT_BOT_COST: u64 = 50;

// --- State ---

#[derive(Resource)]
struct ShopState {
    pickaxe_tier: usize,
    bag_tier: usize,
    speed_tier: usize,
}

impl Default for ShopState {
    fn default() -> Self {
        Self {
            pickaxe_tier: 0,
            bag_tier: 0,
            speed_tier: 0,
        }
    }
}

// --- Components ---

#[derive(Component)]
struct ShopZone;

#[derive(Component)]
struct ShopModal;

#[derive(Component)]
struct ShopContentText;

// --- Systems ---

fn spawn_shop_zone(mut commands: Commands) {
    commands.spawn((
        ShopZone,
        Sprite {
            color: Color::srgb(1.0, 0.42, 0.62),
            custom_size: Some(Vec2::splat(SHOP_ZONE_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, 40.0, -2.0),
    ));
}

fn shop_toggle(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    shop_query: Query<&Transform, With<ShopZone>>,
    modal_query: Query<Entity, With<ShopModal>>,
    mut shop_open: ResMut<ShopOpen>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Enter);

    if !toggle && !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) && shop_open.0 {
        close_shop(&mut commands, &modal_query, &mut shop_open);
        return;
    }

    if toggle {
        if shop_open.0 {
            close_shop(&mut commands, &modal_query, &mut shop_open);
        } else {
            let player_pos = player_query.single().translation.truncate();
            let shop_pos = shop_query.single().translation.truncate();
            let half = SHOP_ZONE_SIZE / 2.0;
            let in_zone = (player_pos.x - shop_pos.x).abs() <= half
                && (player_pos.y - shop_pos.y).abs() <= half;

            if in_zone {
                shop_open.0 = true;
                spawn_shop_modal(&mut commands);
            }
        }
    }
}

fn close_shop(
    commands: &mut Commands,
    modal_query: &Query<Entity, With<ShopModal>>,
    shop_open: &mut ResMut<ShopOpen>,
) {
    shop_open.0 = false;
    for entity in modal_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn shop_buy(
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    mut wallet: ResMut<Wallet>,
    mut stats: ResMut<PlayerStats>,
    mut shop_state: ResMut<ShopState>,
    mut miner_inv: ResMut<MinerInventory>,
    mut bot_inv: ResMut<BotInventory>,
    mut text_query: Query<&mut Text, With<ShopContentText>>,
) {
    if !shop_open.0 {
        return;
    }

    let mut bought = false;

    // [1] Pioche
    if keys.just_pressed(KeyCode::Digit1) {
        let next = shop_state.pickaxe_tier + 1;
        if next < PICKAXE_TIERS.len() && wallet.money >= PICKAXE_TIERS[next].cost {
            wallet.money -= PICKAXE_TIERS[next].cost;
            shop_state.pickaxe_tier = next;
            stats.mining_power = PICKAXE_TIERS[next].value;
            bought = true;
        }
    }

    // [2] Sac
    if keys.just_pressed(KeyCode::Digit2) {
        let next = shop_state.bag_tier + 1;
        if next < BAG_TIERS.len() && wallet.money >= BAG_TIERS[next].cost {
            wallet.money -= BAG_TIERS[next].cost;
            shop_state.bag_tier = next;
            stats.inventory_capacity = BAG_TIERS[next].value as u32;
            bought = true;
        }
    }

    // [3] Vitesse
    if keys.just_pressed(KeyCode::Digit3) {
        let next = shop_state.speed_tier + 1;
        if next < SPEED_TIERS.len() && wallet.money >= SPEED_TIERS[next].cost {
            wallet.money -= SPEED_TIERS[next].cost;
            shop_state.speed_tier = next;
            stats.walk_speed = 150.0 * SPEED_TIERS[next].value;
            bought = true;
        }
    }

    // [4] Auto-mineur
    if keys.just_pressed(KeyCode::Digit4) {
        if wallet.money >= AUTO_MINER_COST {
            wallet.money -= AUTO_MINER_COST;
            miner_inv.count += 1;
            bought = true;
        }
    }

    // [5] Bot éclaireur
    if keys.just_pressed(KeyCode::Digit5) {
        if wallet.money >= SCOUT_BOT_COST {
            wallet.money -= SCOUT_BOT_COST;
            bot_inv.count += 1;
            bought = true;
        }
    }

    // Toujours rafraîchir le texte quand le shop est ouvert
    if let Ok(mut text) = text_query.get_single_mut() {
        *text = Text::new(build_shop_text(&shop_state, &wallet, &miner_inv, &bot_inv));
    }
}

fn build_shop_text(state: &ShopState, wallet: &Wallet, miner_inv: &MinerInventory, bot_inv: &BotInventory) -> String {
    let mut lines = String::from("--- SHOP ---\n[E/Echap] Fermer\n\n");

    // Pioche
    let current_pick = &PICKAXE_TIERS[state.pickaxe_tier];
    lines.push_str(&format!("  Actuel: {} (puissance {})\n", current_pick.name, current_pick.value));
    if state.pickaxe_tier + 1 < PICKAXE_TIERS.len() {
        let next = &PICKAXE_TIERS[state.pickaxe_tier + 1];
        let affordable = if wallet.money >= next.cost { "" } else { " (pas assez)" };
        lines.push_str(&format!("[1] {} — puissance {} — {} c{}\n", next.name, next.value, next.cost, affordable));
    } else {
        lines.push_str("[1] MAX\n");
    }

    lines.push('\n');

    // Sac
    let current_bag = &BAG_TIERS[state.bag_tier];
    lines.push_str(&format!("  Actuel: {} (capacite {})\n", current_bag.name, current_bag.value as u32));
    if state.bag_tier + 1 < BAG_TIERS.len() {
        let next = &BAG_TIERS[state.bag_tier + 1];
        let affordable = if wallet.money >= next.cost { "" } else { " (pas assez)" };
        lines.push_str(&format!("[2] {} — capacite {} — {} c{}\n", next.name, next.value as u32, next.cost, affordable));
    } else {
        lines.push_str("[2] MAX\n");
    }

    lines.push('\n');

    // Vitesse
    let current_speed = &SPEED_TIERS[state.speed_tier];
    lines.push_str(&format!("  Actuel: {} (x{})\n", current_speed.name, current_speed.value));
    if state.speed_tier + 1 < SPEED_TIERS.len() {
        let next = &SPEED_TIERS[state.speed_tier + 1];
        let affordable = if wallet.money >= next.cost { "" } else { " (pas assez)" };
        lines.push_str(&format!("[3] {} — x{} — {} c{}\n", next.name, next.value, next.cost, affordable));
    } else {
        lines.push_str("[3] MAX\n");
    }

    lines.push('\n');

    // Auto-mineur
    lines.push_str(&format!("  Mineurs en stock: {}\n", miner_inv.count));
    let affordable = if wallet.money >= AUTO_MINER_COST { "" } else { " (pas assez)" };
    lines.push_str(&format!("[4] Auto-mineur — {} c{}\n", AUTO_MINER_COST, affordable));

    lines.push('\n');

    // Bot éclaireur
    lines.push_str(&format!("  Bots en stock: {}\n", bot_inv.count));
    let affordable = if wallet.money >= SCOUT_BOT_COST { "" } else { " (pas assez)" };
    lines.push_str(&format!("[5] Bot eclaireur — {} c{}\n", SCOUT_BOT_COST, affordable));

    lines
}

fn spawn_shop_modal(commands: &mut Commands) {
    commands
        .spawn((
            ShopModal,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(15.0),
                right: Val::Percent(15.0),
                top: Val::Percent(10.0),
                bottom: Val::Percent(10.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.05, 0.15, 0.95)),
        ))
        .with_children(|parent| {
            // Texte vide — sera rempli par shop_buy via is_changed()
            parent.spawn((
                ShopContentText,
                Text::new(""),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}
