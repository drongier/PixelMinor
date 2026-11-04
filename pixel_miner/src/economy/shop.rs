use bevy::prelude::*;

use crate::machines::auto_miner::MinerInventory;
use crate::player::player::Player;
use crate::economy::resources::{
    bag_capacity, pickaxe_power, walk_speed, GameState, PlayerStats, UpgradeTier, Wallet,
    BAG_TIERS, BASE_WALK_SPEED, PICKAXE_TIERS, SPEED_TIERS,
};
use crate::machines::scout_bot::BotInventory;
use crate::world::tiles::{SHOP_ZONE_POS, TILE_SIZE};

const SHOP_ZONE_SIZE: f32 = TILE_SIZE * 2.0;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ShopState::default())
            .add_systems(Startup, spawn_shop_zone)
            .add_systems(OnEnter(GameState::InShop), spawn_shop_modal)
            .add_systems(OnExit(GameState::InShop), despawn_shop_modal)
            .add_systems(
                Update,
                (
                    open_shop.run_if(in_state(GameState::Playing)),
                    (close_shop, shop_buy).run_if(in_state(GameState::InShop)),
                ),
            );
    }
}

// --- Upgrade definitions ---
// Les paliers (PICKAXE_TIERS, BAG_TIERS, SPEED_TIERS) vivent dans resources.rs
// pour que PlayerStats::default() en dérive directement.

const AUTO_MINER_COST: u64 = 100;
const SCOUT_BOT_COST: u64 = 50;

// --- State ---

#[derive(Resource, Default)]
struct ShopState {
    pickaxe_tier: usize,
    bag_tier: usize,
    speed_tier: usize,
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
        Transform::from_xyz(SHOP_ZONE_POS.x, SHOP_ZONE_POS.y, -2.0),
    ));
}

fn toggle_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Enter)
}

/// E / Entrée dans la zone du shop → InShop (le modal est créé par OnEnter).
fn open_shop(
    keys: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    shop_query: Query<&Transform, With<ShopZone>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !toggle_pressed(&keys) {
        return;
    }
    let player_pos = player_query.single().translation.truncate();
    let shop_pos = shop_query.single().translation.truncate();
    let half = SHOP_ZONE_SIZE / 2.0;
    let in_zone = (player_pos.x - shop_pos.x).abs() <= half
        && (player_pos.y - shop_pos.y).abs() <= half;
    if in_zone {
        next_state.set(GameState::InShop);
    }
}

/// E / Entrée / Échap → Playing (le modal est détruit par OnExit).
fn close_shop(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if toggle_pressed(&keys) || keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Playing);
    }
}

fn despawn_shop_modal(mut commands: Commands, modal_query: Query<Entity, With<ShopModal>>) {
    for entity in modal_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Tente d'acheter le palier suivant d'une table. Retourne le nouveau palier si achat.
fn try_upgrade(tiers: &[UpgradeTier], current: usize, wallet: &mut Wallet) -> Option<usize> {
    let next = current + 1;
    let tier = tiers.get(next)?;
    if wallet.money < tier.cost {
        return None;
    }
    wallet.money -= tier.cost;
    Some(next)
}

#[allow(clippy::too_many_arguments)] // système Bevy : une ressource par paramètre
fn shop_buy(
    keys: Res<ButtonInput<KeyCode>>,
    mut wallet: ResMut<Wallet>,
    mut stats: ResMut<PlayerStats>,
    mut shop_state: ResMut<ShopState>,
    mut miner_inv: ResMut<MinerInventory>,
    mut bot_inv: ResMut<BotInventory>,
    mut text_query: Query<&mut Text, With<ShopContentText>>,
) {
    let mut bought = false;

    // [1] Pioche
    if keys.just_pressed(KeyCode::Digit1) {
        if let Some(t) = try_upgrade(PICKAXE_TIERS, shop_state.pickaxe_tier, &mut wallet) {
            shop_state.pickaxe_tier = t;
            stats.mining_power = pickaxe_power(t);
            bought = true;
        }
    }

    // [2] Sac
    if keys.just_pressed(KeyCode::Digit2) {
        if let Some(t) = try_upgrade(BAG_TIERS, shop_state.bag_tier, &mut wallet) {
            shop_state.bag_tier = t;
            stats.inventory_capacity = bag_capacity(t);
            bought = true;
        }
    }

    // [3] Vitesse
    if keys.just_pressed(KeyCode::Digit3) {
        if let Some(t) = try_upgrade(SPEED_TIERS, shop_state.speed_tier, &mut wallet) {
            shop_state.speed_tier = t;
            stats.walk_speed = walk_speed(t);
            bought = true;
        }
    }

    // [4] Auto-mineur
    if keys.just_pressed(KeyCode::Digit4) && wallet.money >= AUTO_MINER_COST {
        wallet.money -= AUTO_MINER_COST;
        miner_inv.count += 1;
        bought = true;
    }

    // [5] Bot éclaireur
    if keys.just_pressed(KeyCode::Digit5) && wallet.money >= SCOUT_BOT_COST {
        wallet.money -= SCOUT_BOT_COST;
        bot_inv.count += 1;
        bought = true;
    }

    // Rafraîchir le texte à l'ouverture (texte vide), après un achat,
    // ou si l'argent / les stocks ont changé ailleurs.
    if let Ok(mut text) = text_query.get_single_mut() {
        let needs_refresh = bought
            || text.0.is_empty()
            || wallet.is_changed()
            || miner_inv.is_changed()
            || bot_inv.is_changed();
        if needs_refresh {
            *text = Text::new(build_shop_text(&shop_state, &wallet, &miner_inv, &bot_inv));
        }
    }
}

fn affordable_suffix(wallet: &Wallet, cost: u64) -> &'static str {
    if wallet.money >= cost { "" } else { " (pas assez)" }
}

/// Écrit la ligne "Actuel" + la ligne d'achat (ou MAX) pour une table de paliers.
fn push_tier_section(
    lines: &mut String,
    key: u8,
    tiers: &[UpgradeTier],
    current: usize,
    wallet: &Wallet,
    describe: impl Fn(&UpgradeTier) -> String,
) {
    let cur = &tiers[current];
    lines.push_str(&format!("  Actuel: {} ({})\n", cur.name, describe(cur)));
    match tiers.get(current + 1) {
        Some(next) => lines.push_str(&format!(
            "[{}] {} — {} — {} c{}\n",
            key,
            next.name,
            describe(next),
            next.cost,
            affordable_suffix(wallet, next.cost)
        )),
        None => lines.push_str(&format!("[{}] MAX\n", key)),
    }
    lines.push('\n');
}

fn build_shop_text(state: &ShopState, wallet: &Wallet, miner_inv: &MinerInventory, bot_inv: &BotInventory) -> String {
    let mut lines = String::from("--- SHOP ---\n[E/Echap] Fermer\n\n");

    push_tier_section(&mut lines, 1, PICKAXE_TIERS, state.pickaxe_tier, wallet, |t| {
        format!("puissance {}", t.value)
    });
    push_tier_section(&mut lines, 2, BAG_TIERS, state.bag_tier, wallet, |t| {
        format!("capacite {}", t.value as u32)
    });
    push_tier_section(&mut lines, 3, SPEED_TIERS, state.speed_tier, wallet, |t| {
        format!("x{} = {:.0}", t.value, BASE_WALK_SPEED * t.value)
    });

    // Auto-mineur
    lines.push_str(&format!("  Mineurs en stock: {}\n", miner_inv.count));
    lines.push_str(&format!(
        "[4] Auto-mineur — {} c{}\n",
        AUTO_MINER_COST,
        affordable_suffix(wallet, AUTO_MINER_COST)
    ));

    lines.push('\n');

    // Bot éclaireur
    lines.push_str(&format!("  Bots en stock: {}\n", bot_inv.count));
    lines.push_str(&format!(
        "[5] Bot eclaireur — {} c{}\n",
        SCOUT_BOT_COST,
        affordable_suffix(wallet, SCOUT_BOT_COST)
    ));

    lines
}

fn spawn_shop_modal(mut commands: Commands) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text_for(state: &ShopState, money: u64) -> String {
        build_shop_text(
            state,
            &Wallet { money },
            &MinerInventory::default(),
            &BotInventory::default(),
        )
    }

    #[test]
    fn shop_shows_max_at_max_tier() {
        let state = ShopState {
            pickaxe_tier: PICKAXE_TIERS.len() - 1,
            bag_tier: BAG_TIERS.len() - 1,
            speed_tier: SPEED_TIERS.len() - 1,
        };
        let text = text_for(&state, 0);
        assert!(text.contains("[1] MAX"));
        assert!(text.contains("[2] MAX"));
        assert!(text.contains("[3] MAX"));
    }

    #[test]
    fn shop_shows_next_tier_at_start() {
        let text = text_for(&ShopState::default(), 0);
        assert!(!text.contains("MAX"));
        assert!(text.contains(PICKAXE_TIERS[0].name));
        assert!(text.contains(PICKAXE_TIERS[1].name));
        assert!(text.contains("(pas assez)"));
    }

    #[test]
    fn try_upgrade_respects_money_and_max() {
        let mut wallet = Wallet { money: PICKAXE_TIERS[1].cost };
        assert_eq!(try_upgrade(PICKAXE_TIERS, 0, &mut wallet), Some(1));
        assert_eq!(wallet.money, 0);
        assert_eq!(try_upgrade(PICKAXE_TIERS, 1, &mut wallet), None);
        let mut rich = Wallet { money: u64::MAX };
        assert_eq!(try_upgrade(PICKAXE_TIERS, PICKAXE_TIERS.len() - 1, &mut rich), None);
        assert_eq!(rich.money, u64::MAX);
    }

    #[test]
    fn escape_in_shop_returns_to_playing_and_despawns_modal() {
        use bevy::state::app::StatesPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(OnEnter(GameState::InShop), spawn_shop_modal)
            .add_systems(OnExit(GameState::InShop), despawn_shop_modal)
            .add_systems(Update, close_shop.run_if(in_state(GameState::InShop)));

        app.world_mut().resource_mut::<NextState<GameState>>().set(GameState::InShop);
        app.update();
        let modals = |app: &mut App| app.world_mut().query::<&ShopModal>().iter(app.world()).count();
        assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::InShop);
        assert_eq!(modals(&mut app), 1);

        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
        app.update(); // close_shop demande Playing
        app.update(); // transition appliquée, OnExit despawn
        assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::Playing);
        assert_eq!(modals(&mut app), 0);
    }
}
