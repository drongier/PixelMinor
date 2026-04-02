use bevy::prelude::*;
use rand::Rng;

use crate::world::tiles::{GridPos, TileMinedEvent, TileType, BASE_RADIUS, GRID_SIZE, TILE_SIZE};

pub struct DepositPlugin;

impl Plugin for DepositPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_deposits)
            .add_systems(Update, reveal_deposits_on_mine);
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum DepositType {
    Ferrous,
    Precious,
    Gems,
    Mythic,
}

impl DepositType {
    pub fn label(&self) -> &str {
        match self {
            DepositType::Ferrous  => "Gisement ferreux",
            DepositType::Precious => "Gisement précieux",
            DepositType::Gems     => "Gisement de gemmes",
            DepositType::Mythic   => "Gisement mythique",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            DepositType::Ferrous  => Color::srgb(0.45, 0.25, 0.12),
            DepositType::Precious => Color::srgb(0.50, 0.42, 0.10),
            DepositType::Gems     => Color::srgb(0.10, 0.35, 0.40),
            DepositType::Mythic   => Color::srgb(0.30, 0.15, 0.45),
        }
    }

    pub fn drop_table(&self) -> &[(TileType, f32)] {
        match self {
            DepositType::Ferrous => &[
                (TileType::Iron, 0.70),
                (TileType::Coal, 0.90),
                (TileType::Copper, 0.98),
                (TileType::Silver, 1.00),
            ],
            DepositType::Precious => &[
                (TileType::Silver, 0.50),
                (TileType::Gold, 0.80),
                (TileType::Copper, 0.95),
                (TileType::Diamond, 1.00),
            ],
            DepositType::Gems => &[
                (TileType::Diamond, 0.40),
                (TileType::Ruby, 0.75),
                (TileType::Gold, 0.95),
                (TileType::Mythril, 1.00),
            ],
            DepositType::Mythic => &[
                (TileType::Mythril, 0.45),
                (TileType::Obsidian, 0.75),
                (TileType::Diamond, 0.90),
                (TileType::Ruby, 1.00),
            ],
        }
    }
}

/// Choisit un type de gisement selon la zone de profondeur
fn pick_deposit_type(distance: i32, rng: &mut impl Rng) -> Option<DepositType> {
    let roll: f32 = rng.gen();
    match distance {
        3..=20 => Some(DepositType::Ferrous),
        21..=45 => {
            if roll < 0.70 { Some(DepositType::Ferrous) }
            else { Some(DepositType::Precious) }
        }
        46..=75 => {
            if roll < 0.40 { Some(DepositType::Precious) }
            else { Some(DepositType::Gems) }
        }
        76..=110 => {
            if roll < 0.40 { Some(DepositType::Gems) }
            else { Some(DepositType::Mythic) }
        }
        111.. => Some(DepositType::Mythic),
        _ => None,
    }
}

#[derive(Component)]
pub struct Deposit {
    pub deposit_type: DepositType,
    pub revealed: bool,
}

const DEPOSIT_COUNT: usize = 80;
const HIDDEN_DEPOSIT_COLOR: Color = Color::srgb(0.15, 0.12, 0.10);

fn spawn_deposits(mut commands: Commands) {
    let half = GRID_SIZE / 2;
    let mut rng = rand::thread_rng();
    let mut occupied: Vec<GridPos> = Vec::new();

    for _ in 0..DEPOSIT_COUNT {
        // Choisir un point central aléatoire hors de la base
        let cx = rng.gen_range(-half..half);
        let cy = rng.gen_range(-half..half);
        let center = GridPos { x: cx, y: cy };
        let dist = center.distance_from_center();

        if dist <= BASE_RADIUS {
            continue;
        }

        let deposit_type = match pick_deposit_type(dist, &mut rng) {
            Some(dt) => dt,
            None => continue,
        };

        // Faire croître un cluster de 2-4 tiles depuis le centre
        let cluster_size = rng.gen_range(2..=4);
        let mut cluster = vec![center];
        let mut candidates = center.neighbors().to_vec();

        while cluster.len() < cluster_size && !candidates.is_empty() {
            let idx = rng.gen_range(0..candidates.len());
            let pos = candidates[idx];
            candidates.swap_remove(idx);

            let d = pos.distance_from_center();
            if d <= BASE_RADIUS || d >= half || occupied.contains(&pos) || cluster.contains(&pos) {
                continue;
            }

            cluster.push(pos);
            for n in pos.neighbors() {
                if !cluster.contains(&n) && !candidates.contains(&n) {
                    candidates.push(n);
                }
            }
        }

        // Spawner les entités deposit
        for pos in &cluster {
            occupied.push(*pos);
            let world = pos.to_world();

            commands.spawn((
                Deposit {
                    deposit_type: deposit_type.clone(),
                    revealed: false,
                },
                *pos,
                Sprite {
                    color: HIDDEN_DEPOSIT_COLOR,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, -2.0),
            ));
        }
    }
}

fn reveal_deposits_on_mine(
    mut events: EventReader<TileMinedEvent>,
    mut deposit_query: Query<(&GridPos, &mut Deposit, &mut Sprite)>,
) {
    for event in events.read() {
        for (grid_pos, mut deposit, mut sprite) in deposit_query.iter_mut() {
            if *grid_pos == event.pos && !deposit.revealed {
                deposit.revealed = true;
                sprite.color = deposit.deposit_type.color();
            }
        }
    }
}
