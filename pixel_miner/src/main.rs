mod economy;
mod machines;
mod player;
mod ui;
mod world;

use bevy::prelude::*;

use economy::resources::{GameState, Inventory, PlayerStats, Wallet};
use economy::sell::SellPlugin;
use economy::shop::ShopPlugin;
use machines::auto_miner::AutoMinerPlugin;
use machines::conveyor::ConveyorPlugin;
use machines::scout_bot::ScoutBotPlugin;
use player::player::PlayerPlugin;
use ui::hud::HudPlugin;
use world::deposits::DepositPlugin;
use world::map::MapPlugin;
use world::tiles::WorldSeed;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "PixelMiner".to_string(),
                        resolution: (1920.0, 1080.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<GameState>()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Inventory::default())
        .insert_resource(Wallet::default())
        .insert_resource(PlayerStats::default())
        .insert_resource(WorldSeed::from_env_or_random())
        .add_plugins((MapPlugin, DepositPlugin, AutoMinerPlugin, ScoutBotPlugin, ConveyorPlugin, PlayerPlugin, SellPlugin, ShopPlugin, HudPlugin))
        .add_systems(Startup, (spawn_camera, log_world_seed))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Loggé dans un système (et non avant `run`) pour que le LogPlugin soit actif
fn log_world_seed(seed: Res<WorldSeed>) {
    info!(
        "World seed: {} (rejouer avec {}={})",
        seed.0,
        WorldSeed::ENV_VAR,
        seed.0
    );
}
