use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

use crate::world::tiles::{
    GridPos, TileMinedEvent, TileType, WorldSeed, Zone, BASE_RADIUS, GRID_SIZE, TILE_SIZE,
};

pub struct DepositPlugin;

impl Plugin for DepositPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DepositMap>()
            .add_systems(Startup, spawn_deposits)
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
    match Zone::from_distance(distance) {
        Zone::Base => None,
        Zone::Z1 => Some(DepositType::Ferrous),
        Zone::Z2 => {
            if roll < 0.70 { Some(DepositType::Ferrous) }
            else { Some(DepositType::Precious) }
        }
        Zone::Z3 => {
            if roll < 0.40 { Some(DepositType::Precious) }
            else { Some(DepositType::Gems) }
        }
        Zone::Z4 => {
            if roll < 0.40 { Some(DepositType::Gems) }
            else { Some(DepositType::Mythic) }
        }
        Zone::Z5 => Some(DepositType::Mythic),
    }
}

/// Index des gisements par position pour lookup O(1)
#[derive(Resource, Default)]
pub struct DepositMap {
    pub deposits: HashMap<GridPos, Entity>,
}

#[derive(Component)]
pub struct Deposit {
    pub deposit_type: DepositType,
    pub revealed: bool,
}

const DEPOSIT_COUNT: usize = 80;
const HIDDEN_DEPOSIT_COLOR: Color = Color::srgb(0.15, 0.12, 0.10);

/// Plafond de tirages de centres pour éviter une boucle infinie si la carte sature
const MAX_DEPOSIT_ATTEMPTS: usize = DEPOSIT_COUNT * 50;

/// Génération pure (sans ECS) des gisements : liste des cases et de leur type.
/// Garantit l'absence de chevauchement et le respect des bornes de la carte.
pub fn generate_deposits(seed: u64) -> Vec<(GridPos, DepositType)> {
    let half = GRID_SIZE / 2;
    let in_bounds = |p: GridPos| (-half..half).contains(&p.x) && (-half..half).contains(&p.y);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut occupied: HashSet<GridPos> = HashSet::new();
    let mut result: Vec<(GridPos, DepositType)> = Vec::new();
    let mut placed = 0;

    for _ in 0..MAX_DEPOSIT_ATTEMPTS {
        if placed >= DEPOSIT_COUNT {
            break;
        }

        // Choisir un point central aléatoire hors de la base et non occupé
        let center = GridPos {
            x: rng.gen_range(-half..half),
            y: rng.gen_range(-half..half),
        };
        let dist = center.distance_from_center();
        if dist <= BASE_RADIUS || occupied.contains(&center) {
            continue;
        }

        let Some(deposit_type) = pick_deposit_type(dist, &mut rng) else {
            continue;
        };

        // Faire croître un cluster de 2-4 tiles depuis le centre
        let cluster_size = rng.gen_range(2..=4);
        let mut cluster = vec![center];
        let mut candidates = center.neighbors().to_vec();

        while cluster.len() < cluster_size && !candidates.is_empty() {
            let idx = rng.gen_range(0..candidates.len());
            let pos = candidates.swap_remove(idx);

            if pos.distance_from_center() <= BASE_RADIUS
                || !in_bounds(pos)
                || occupied.contains(&pos)
                || cluster.contains(&pos)
            {
                continue;
            }

            cluster.push(pos);
            for n in pos.neighbors() {
                if !cluster.contains(&n) && !candidates.contains(&n) {
                    candidates.push(n);
                }
            }
        }

        for pos in cluster {
            occupied.insert(pos);
            result.push((pos, deposit_type.clone()));
        }
        placed += 1;
    }

    result
}

fn spawn_deposits(
    mut commands: Commands,
    mut deposit_map: ResMut<DepositMap>,
    seed: Res<WorldSeed>,
) {
    for (pos, deposit_type) in generate_deposits(seed.0.wrapping_add(1)) {
        let world = pos.to_world();

        let entity = commands.spawn((
            Deposit {
                deposit_type,
                revealed: false,
            },
            pos,
            Sprite {
                color: HIDDEN_DEPOSIT_COLOR,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, -2.0),
        )).id();
        deposit_map.deposits.insert(pos, entity);
    }
}

fn reveal_deposits_on_mine(
    mut events: EventReader<TileMinedEvent>,
    deposit_map: Res<DepositMap>,
    mut deposit_query: Query<(&mut Deposit, &mut Sprite)>,
) {
    for event in events.read() {
        let Some(&entity) = deposit_map.deposits.get(&event.pos) else {
            continue;
        };
        if let Ok((mut deposit, mut sprite)) = deposit_query.get_mut(entity) {
            if !deposit.revealed {
                deposit.revealed = true;
                sprite.color = deposit.deposit_type.color();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 12345;

    #[test]
    fn deposits_do_not_overlap() {
        let deposits = generate_deposits(SEED);
        let unique: HashSet<GridPos> = deposits.iter().map(|(p, _)| *p).collect();
        assert_eq!(unique.len(), deposits.len());
    }

    #[test]
    fn deposits_stay_in_bounds_and_out_of_base() {
        let half = GRID_SIZE / 2;
        for (p, _) in generate_deposits(SEED) {
            assert!((-half..half).contains(&p.x) && (-half..half).contains(&p.y), "{p:?}");
            assert!(p.distance_from_center() > BASE_RADIUS, "{p:?}");
        }
    }

    #[test]
    fn deposits_have_multi_tile_clusters_beyond_zone_3() {
        let deposits = generate_deposits(SEED);
        let by_pos: HashMap<GridPos, DepositType> = deposits.iter().cloned().collect();
        let far_with_neighbor = deposits
            .iter()
            .filter(|(p, _)| p.distance_from_center() > 75)
            .filter(|(p, t)| p.neighbors().iter().any(|n| by_pos.get(n) == Some(t)))
            .count();
        assert!(far_with_neighbor > 0, "no multi-tile cluster beyond distance 75");
    }

    #[test]
    fn deposits_place_requested_count_and_are_deterministic() {
        let a = generate_deposits(SEED);
        assert_eq!(a, generate_deposits(SEED));
        // Chaque gisement fait au moins 1 case ; au moins DEPOSIT_COUNT cases au total
        assert!(a.len() >= DEPOSIT_COUNT);
    }
}
