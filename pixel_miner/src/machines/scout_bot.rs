use bevy::prelude::*;
use std::collections::HashSet;

use crate::economy::resources::{PlayerStats, ShopOpen};
use crate::player::player::Player;
use crate::world::deposits::{Deposit, DepositMap, DepositType};
use crate::world::tiles::{GridPos, Tile, TileMap, TileMinedEvent, TILE_SIZE};

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
    /// Avancement (0..1) entre la case courante et la suivante
    progress: f32,
}

const BOT_MINING_POWER_RATIO: f32 = 1.0;
/// Vitesse de déplacement du bot sur sol dégagé (tiles / seconde)
const BOT_SPEED: f32 = 6.0;

type BotQuery<'w, 's> = Query<'w, 's, (Entity, &'static mut ScoutBot, &'static mut GridPos, &'static mut Transform)>;
type BotTileQuery<'w, 's> = Query<'w, 's, &'static mut Tile, (Without<ScoutBot>, Without<Player>)>;
type BotDepositQuery<'w, 's> =
    Query<'w, 's, (&'static mut Deposit, &'static mut Sprite), (Without<ScoutBot>, Without<Tile>)>;

/// Prochaine case (4-voisinage) en direction de `to`, axe le plus long d'abord.
fn step_toward(from: GridPos, to: GridPos) -> GridPos {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dx == 0 && dy == 0 {
        from
    } else if dx.abs() >= dy.abs() {
        GridPos { x: from.x + dx.signum(), y: from.y }
    } else {
        GridPos { x: from.x, y: from.y + dy.signum() }
    }
}

/// Flood fill 4-voisins depuis `start`. `start` est toujours inclus ;
/// les autres cases le sont si `is_member` renvoie vrai.
pub fn flood_cluster(start: GridPos, is_member: impl Fn(GridPos) -> bool) -> Vec<GridPos> {
    let mut visited = HashSet::from([start]);
    let mut stack = vec![start];
    let mut cluster = Vec::new();
    while let Some(pos) = stack.pop() {
        cluster.push(pos);
        for n in pos.neighbors() {
            if !visited.contains(&n) && is_member(n) {
                visited.insert(n);
                stack.push(n);
            }
        }
    }
    cluster
}

/// Cases du gisement caché contenant `start` (même type, non révélées).
fn hidden_cluster(
    start: GridPos,
    deposit_map: &DepositMap,
    deposit_of: impl Fn(Entity) -> Option<(DepositType, bool)>,
) -> Vec<GridPos> {
    let Some((start_type, _)) = deposit_map.deposits.get(&start).and_then(|&e| deposit_of(e)) else {
        return Vec::new();
    };
    flood_cluster(start, |pos| {
        deposit_map
            .deposits
            .get(&pos)
            .and_then(|&e| deposit_of(e))
            .is_some_and(|(t, revealed)| t == start_type && !revealed)
    })
}

#[allow(clippy::too_many_arguments, reason = "Bevy system: each param is a distinct resource/query")]
fn deploy_bot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    shop_open: Res<ShopOpen>,
    stats: Res<PlayerStats>,
    mut bot_inv: ResMut<BotInventory>,
    player_query: Query<&Transform, With<Player>>,
    deposit_map: Res<DepositMap>,
    deposit_query: Query<(&GridPos, &Deposit)>,
    bots: Query<&ScoutBot>,
) {
    if !keys.just_pressed(KeyCode::KeyG) || shop_open.0 || bot_inv.count == 0 {
        return;
    }

    let player_pos = player_query.single().translation.truncate();
    let player_grid = GridPos::from_world(player_pos);

    // Les gisements déjà visés par un bot (cluster entier) sont exclus
    let deposit_of = |e: Entity| deposit_query.get(e).ok().map(|(_, d)| (d.deposit_type.clone(), d.revealed));
    let targeted: HashSet<GridPos> = bots
        .iter()
        .flat_map(|bot| hidden_cluster(bot.target, &deposit_map, deposit_of))
        .collect();

    // Gisement non-révélé et non-visé le plus proche
    let Some(target) = deposit_query
        .iter()
        .filter(|(pos, deposit)| !deposit.revealed && !targeted.contains(pos))
        .min_by_key(|(pos, _)| pos.manhattan(&player_grid))
        .map(|(pos, _)| *pos)
    else {
        return;
    };

    bot_inv.count -= 1;

    commands.spawn((
        ScoutBot {
            target,
            mining_power: stats.mining_power * BOT_MINING_POWER_RATIO,
            progress: 0.0,
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

#[allow(clippy::too_many_arguments, reason = "Bevy system: each param is a distinct resource/query")]
fn bot_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut bot_query: BotQuery,
    tile_map: Res<TileMap>,
    deposit_map: Res<DepositMap>,
    mut tile_query: BotTileQuery,
    mut deposit_query: BotDepositQuery,
    mut mine_events: EventWriter<TileMinedEvent>,
) {
    for (bot_entity, mut bot, mut bot_grid, mut bot_transform) in bot_query.iter_mut() {
        // Arrivé : révéler tout le gisement (cases connexes cachées de même type)
        if *bot_grid == bot.target {
            let cluster = hidden_cluster(bot.target, &deposit_map, |e| {
                deposit_query.get(e).ok().map(|(d, _)| (d.deposit_type.clone(), d.revealed))
            });
            for pos in cluster {
                let Some(&entity) = deposit_map.deposits.get(&pos) else {
                    continue;
                };
                if let Ok((mut deposit, mut sprite)) = deposit_query.get_mut(entity) {
                    if !deposit.revealed {
                        deposit.revealed = true;
                        sprite.color = deposit.deposit_type.color();
                    }
                }
            }
            commands.entity(bot_entity).despawn();
            continue;
        }

        let next_grid = step_toward(*bot_grid, bot.target);

        // Tile à miner devant le bot ? (Err si déjà despawnée)
        let blocking = tile_map
            .tiles
            .get(&next_grid)
            .and_then(|&entity| tile_query.get_mut(entity).ok().map(|tile| (entity, tile)));

        if let Some((entity, mut tile)) = blocking {
            // Miner la tile, le bot reste sur sa case
            bot.progress = 0.0;
            tile.hp -= bot.mining_power * time.delta_secs();
            if tile.hp <= 0.0 {
                mine_events.send(TileMinedEvent { pos: next_grid });
                commands.entity(entity).despawn();
            }
            let world = bot_grid.to_world();
            bot_transform.translation.x = world.x;
            bot_transform.translation.y = world.y;
            continue;
        }

        // Sol dégagé : avancer à vitesse constante, au plus une case par frame
        bot.progress += BOT_SPEED * time.delta_secs();
        let world = if bot.progress >= 1.0 {
            bot.progress = 0.0;
            *bot_grid = next_grid;
            next_grid.to_world()
        } else {
            bot_grid.to_world().lerp(next_grid.to_world(), bot.progress)
        };
        bot_transform.translation.x = world.x;
        bot_transform.translation.y = world.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(x: i32, y: i32) -> GridPos {
        GridPos { x, y }
    }

    #[test]
    fn step_toward_moves_along_longest_axis() {
        assert_eq!(step_toward(g(0, 0), g(5, 2)), g(1, 0));
        assert_eq!(step_toward(g(0, 0), g(1, -4)), g(0, -1));
        assert_eq!(step_toward(g(3, 3), g(3, 3)), g(3, 3));
    }

    #[test]
    fn flood_cluster_collects_connected_members_only() {
        let members: HashSet<GridPos> = [g(1, 0), g(2, 0), g(2, 1), g(5, 5), g(1, 1)].into();
        // (1,1) est connecté via (1,0) ; (5,5) est isolé
        let mut cluster = flood_cluster(g(0, 0), |p| members.contains(&p));
        cluster.sort_by_key(|p| (p.x, p.y));
        assert_eq!(cluster, vec![g(0, 0), g(1, 0), g(1, 1), g(2, 0), g(2, 1)]);
    }

    #[test]
    fn flood_cluster_ignores_diagonals() {
        let members: HashSet<GridPos> = [g(1, 1)].into();
        assert_eq!(flood_cluster(g(0, 0), |p| members.contains(&p)), vec![g(0, 0)]);
    }
}
