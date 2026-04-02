use bevy::prelude::*;

use crate::world::deposits::Deposit;
use crate::player::player::Player;
use crate::economy::resources::{PlayerStats, ShopOpen};
use crate::world::tiles::{GridPos, Tile, TileMinedEvent, TILE_SIZE};

pub struct ScoutBotPlugin;

impl Plugin for ScoutBotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotInventory::default())
            .add_systems(Update, (deploy_bot, bot_movement));
    }
}

#[derive(Resource, Default)]
pub struct BotInventory {
    pub count: u32,
}

#[derive(Component)]
pub struct ScoutBot {
    target: GridPos,
    mining_power: f32,
}

const BOT_MINING_POWER_RATIO: f32 = 1.0;

fn deploy_bot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    stats: Res<PlayerStats>,
    mut bot_inv: ResMut<BotInventory>,
    player_query: Query<&Transform, With<Player>>,
    deposit_query: Query<(&GridPos, &Deposit)>,
) {
    if !keys.just_pressed(KeyCode::KeyG) || shop_open.0 || bot_inv.count == 0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();
    let player_grid = GridPos {
        x: (player_pos.x / TILE_SIZE).round() as i32,
        y: (player_pos.y / TILE_SIZE).round() as i32,
    };

    // Trouver le gisement non-révélé le plus proche
    let mut best: Option<(GridPos, i32)> = None;
    for (grid_pos, deposit) in deposit_query.iter() {
        if deposit.revealed {
            continue;
        }
        let dist = (grid_pos.x - player_grid.x).abs() + (grid_pos.y - player_grid.y).abs();
        if best.is_none() || dist < best.as_ref().unwrap().1 {
            best = Some((*grid_pos, dist));
        }
    }

    let target = match best {
        Some((pos, _)) => pos,
        None => return,
    };

    bot_inv.count -= 1;

    commands.spawn((
        ScoutBot {
            target,
            mining_power: stats.mining_power * BOT_MINING_POWER_RATIO,
        },
        player_grid,
        Sprite {
            color: Color::srgb(0.2, 0.9, 0.2),
            custom_size: Some(Vec2::splat(TILE_SIZE * 0.5)),
            ..default()
        },
        Transform::from_xyz(player_pos.x, player_pos.y, 0.5),
    ));
}

fn bot_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut bot_query: Query<(Entity, &mut ScoutBot, &mut GridPos, &mut Transform)>,
    mut tile_query: Query<(Entity, &GridPos, &mut Tile), (Without<ScoutBot>, Without<Player>)>,
    mut deposit_query: Query<(&GridPos, &mut Deposit, &mut Sprite), (Without<ScoutBot>, Without<Tile>)>,
    mut mine_events: EventWriter<TileMinedEvent>,
) {
    for (bot_entity, mut bot, mut bot_grid, mut bot_transform) in bot_query.iter_mut() {
        // Vérifier si le bot est arrivé à destination
        if *bot_grid == bot.target {
            // Révéler le gisement
            for (deposit_pos, mut deposit, mut sprite) in deposit_query.iter_mut() {
                if *deposit_pos == bot.target && !deposit.revealed {
                    deposit.revealed = true;
                    sprite.color = deposit.deposit_type.color();
                }
            }
            commands.entity(bot_entity).despawn();
            continue;
        }

        // Déterminer la prochaine tile vers la cible
        let dx = bot.target.x - bot_grid.x;
        let dy = bot.target.y - bot_grid.y;
        let next_grid = if dx.abs() >= dy.abs() {
            GridPos {
                x: bot_grid.x + dx.signum(),
                y: bot_grid.y,
            }
        } else {
            GridPos {
                x: bot_grid.x,
                y: bot_grid.y + dy.signum(),
            }
        };

        // Chercher s'il y a une tile à miner à cette position
        let mut blocking_entity = None;
        for (entity, tile_pos, tile) in tile_query.iter() {
            if *tile_pos == next_grid {
                blocking_entity = Some((entity, tile.hp));
                break;
            }
        }

        if let Some((entity, _)) = blocking_entity {
            // Miner la tile
            if let Ok((_, _, mut tile)) = tile_query.get_mut(entity) {
                tile.hp -= bot.mining_power * time.delta_secs();
                if tile.hp <= 0.0 {
                    mine_events.send(TileMinedEvent { pos: next_grid });
                    commands.entity(entity).despawn();
                }
            }
        } else {
            // Pas de tile bloquante, avancer
            *bot_grid = next_grid;
            let world = next_grid.to_world();
            bot_transform.translation.x = world.x;
            bot_transform.translation.y = world.y;
        }
    }
}
