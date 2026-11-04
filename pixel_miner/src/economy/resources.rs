use bevy::prelude::*;
use std::collections::HashMap;

use crate::world::tiles::TileType;

#[derive(Resource, Default)]
pub struct Inventory {
    pub items: HashMap<TileType, u32>,
}

#[derive(Resource)]
pub struct Wallet {
    pub money: u64,
}

impl Default for Wallet {
    fn default() -> Self {
        Self { money: 500 }
    }
}

/// État de jeu. Seuls les systèmes d'entrée joueur sont filtrés par cet état :
/// la production (mineurs, tapis, bots) continue pendant que le shop est ouvert.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Playing,
    InShop,
}

// --- Upgrade tiers (source unique de vérité pour les stats du joueur) ---

/// Un palier d'amélioration. Le palier 0 de chaque table = stats de départ.
pub struct UpgradeTier {
    pub name: &'static str,
    pub value: f32,
    pub cost: u64,
}

/// Vitesse de marche de base (multipliée par `SPEED_TIERS[i].value`).
pub const BASE_WALK_SPEED: f32 = 250.0;

pub const PICKAXE_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Pioche en bois",   value: 5.0,  cost: 0 },
    UpgradeTier { name: "Pioche en pierre", value: 8.0,  cost: 50 },
    UpgradeTier { name: "Pioche en fer",    value: 12.0, cost: 200 },
    UpgradeTier { name: "Pioche en or",     value: 18.0, cost: 1000 },
    UpgradeTier { name: "Pioche diamant",   value: 28.0, cost: 5000 },
];

pub const BAG_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Poches",    value: 50.0,  cost: 0 },
    UpgradeTier { name: "Petit sac", value: 80.0,  cost: 100 },
    UpgradeTier { name: "Sac a dos", value: 120.0, cost: 500 },
    UpgradeTier { name: "Chariot",   value: 200.0, cost: 2000 },
    UpgradeTier { name: "Wagon",     value: 350.0, cost: 10000 },
];

pub const SPEED_TIERS: &[UpgradeTier] = &[
    UpgradeTier { name: "Normal",      value: 1.0, cost: 0 },
    UpgradeTier { name: "Rapide",      value: 1.3, cost: 300 },
    UpgradeTier { name: "Tres rapide", value: 1.6, cost: 1500 },
    UpgradeTier { name: "Frenetique",  value: 2.0, cost: 8000 },
];

pub fn pickaxe_power(tier: usize) -> f32 {
    PICKAXE_TIERS[tier].value
}

pub fn bag_capacity(tier: usize) -> u32 {
    BAG_TIERS[tier].value as u32
}

pub fn walk_speed(tier: usize) -> f32 {
    (BASE_WALK_SPEED * SPEED_TIERS[tier].value).round()
}

#[derive(Resource, PartialEq, Debug)]
pub struct PlayerStats {
    pub mining_power: f32,
    pub walk_speed: f32,
    pub inventory_capacity: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            mining_power: pickaxe_power(0),
            walk_speed: walk_speed(0),
            inventory_capacity: bag_capacity(0),
        }
    }
}

impl Inventory {
    pub fn total_count(&self) -> u32 {
        self.items.values().sum()
    }

    pub fn is_full(&self, stats: &PlayerStats) -> bool {
        self.total_count() >= stats.inventory_capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_strictly_increasing(name: &str, tiers: &[UpgradeTier]) {
        assert!(!tiers.is_empty(), "{name}: table vide");
        assert_eq!(tiers[0].cost, 0, "{name}: le palier 0 doit etre gratuit");
        for w in tiers.windows(2) {
            assert!(w[1].value > w[0].value, "{name}: {} n'ameliore pas {}", w[1].name, w[0].name);
            assert!(w[1].cost > w[0].cost, "{name}: cout de {} non croissant", w[1].name);
        }
    }

    #[test]
    fn tiers_strictly_increasing() {
        assert_strictly_increasing("pioche", PICKAXE_TIERS);
        assert_strictly_increasing("sac", BAG_TIERS);
        assert_strictly_increasing("vitesse", SPEED_TIERS);
    }

    #[test]
    fn tier_zero_matches_default_stats() {
        let s = PlayerStats::default();
        assert_eq!(s.mining_power, 5.0);
        assert_eq!(s.walk_speed, 250.0);
        assert_eq!(s.inventory_capacity, 50);
        assert_eq!(s.mining_power, PICKAXE_TIERS[0].value);
        assert_eq!(s.inventory_capacity, BAG_TIERS[0].value as u32);
        assert_eq!(s.walk_speed, walk_speed(0));
        for i in 1..SPEED_TIERS.len() {
            assert!(walk_speed(i) > walk_speed(i - 1));
        }
    }

    #[test]
    fn inventory_full_detection() {
        let stats = PlayerStats::default();
        let mut inv = Inventory::default();
        assert!(!inv.is_full(&stats));
        inv.items.insert(TileType::Dirt, stats.inventory_capacity);
        assert!(inv.is_full(&stats));
    }
}
