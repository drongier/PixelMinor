use bevy::prelude::*;
use std::collections::HashMap;

pub const TILE_SIZE: f32 = 16.0;
pub const GRID_SIZE: i32 = 150;
pub const BASE_RADIUS: i32 = 2;

/// Position monde du centre de la zone de vente
pub const SELL_ZONE_POS: Vec2 = Vec2::new(0.0, -40.0);
/// Position monde du centre de la zone du shop
pub const SHOP_ZONE_POS: Vec2 = Vec2::new(0.0, 40.0);

pub const HIDDEN_COLOR: Color = Color::srgb(0.15, 0.12, 0.10);

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TileType {
    Dirt,
    Stone,
    Coal,
    Iron,
    Copper,
    Silver,
    Gold,
    Diamond,
    Ruby,
    Mythril,
    Obsidian,
}

impl TileType {
    pub fn label(&self) -> &str {
        match self {
            TileType::Dirt     => "Terre",
            TileType::Stone    => "Pierre",
            TileType::Coal     => "Charbon",
            TileType::Iron     => "Fer",
            TileType::Copper   => "Cuivre",
            TileType::Silver   => "Argent",
            TileType::Gold     => "Or",
            TileType::Diamond  => "Diamant",
            TileType::Ruby     => "Rubis",
            TileType::Mythril  => "Mythril",
            TileType::Obsidian => "Obsidienne",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            TileType::Dirt     => Color::srgb(0.55, 0.41, 0.08),
            TileType::Stone    => Color::srgb(0.50, 0.50, 0.50),
            TileType::Coal     => Color::srgb(0.20, 0.20, 0.20),
            TileType::Iron     => Color::srgb(0.72, 0.45, 0.20),
            TileType::Copper   => Color::srgb(0.80, 0.50, 0.20),
            TileType::Silver   => Color::srgb(0.75, 0.75, 0.75),
            TileType::Gold     => Color::srgb(1.00, 0.84, 0.00),
            TileType::Diamond  => Color::srgb(0.00, 1.00, 1.00),
            TileType::Ruby     => Color::srgb(1.00, 0.00, 0.25),
            TileType::Mythril  => Color::srgb(0.48, 0.41, 0.93),
            TileType::Obsidian => Color::srgb(0.10, 0.04, 0.18),
        }
    }

    pub fn hp(&self) -> f32 {
        match self {
            TileType::Dirt     => 1.0,
            TileType::Stone    => 2.0,
            TileType::Coal     => 2.0,
            TileType::Iron     => 3.0,
            TileType::Copper   => 3.0,
            TileType::Silver   => 4.0,
            TileType::Gold     => 5.0,
            TileType::Diamond  => 7.0,
            TileType::Ruby     => 7.0,
            TileType::Mythril  => 9.0,
            TileType::Obsidian => 10.0,
        }
    }

    pub fn value(&self) -> u64 {
        match self {
            TileType::Dirt     => 1,
            TileType::Stone    => 2,
            TileType::Coal     => 5,
            TileType::Iron     => 10,
            TileType::Copper   => 12,
            TileType::Silver   => 25,
            TileType::Gold     => 50,
            TileType::Diamond  => 150,
            TileType::Ruby     => 200,
            TileType::Mythril  => 500,
            TileType::Obsidian => 750,
        }
    }

    /// Ordre d'affichage pour le HUD
    pub fn all_ordered() -> &'static [TileType] {
        &[
            TileType::Dirt, TileType::Stone, TileType::Coal,
            TileType::Iron, TileType::Copper, TileType::Silver,
            TileType::Gold, TileType::Diamond, TileType::Ruby,
            TileType::Mythril, TileType::Obsidian,
        ]
    }
}

#[derive(Component)]
pub struct Tile {
    pub tile_type: TileType,
    pub hp: f32,
    pub revealed: bool,
}

/// Position sur la grille
#[derive(Component, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    /// Convertit une position monde en case de grille (arrondi au plus proche)
    pub fn from_world(pos: Vec2) -> GridPos {
        GridPos {
            x: (pos.x / TILE_SIZE).round() as i32,
            y: (pos.y / TILE_SIZE).round() as i32,
        }
    }

    pub fn distance_from_center(self) -> i32 {
        self.x.abs() + self.y.abs()
    }

    /// Distance de Manhattan entre deux cases
    pub fn manhattan(&self, other: &GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn to_world(self) -> Vec2 {
        Vec2::new(self.x as f32 * TILE_SIZE, self.y as f32 * TILE_SIZE)
    }

    pub fn neighbors(self) -> [GridPos; 4] {
        [
            GridPos { x: self.x,     y: self.y + 1 },
            GridPos { x: self.x,     y: self.y - 1 },
            GridPos { x: self.x - 1, y: self.y     },
            GridPos { x: self.x + 1, y: self.y     },
        ]
    }
}

/// Zones de profondeur (distance de Manhattan depuis le centre)
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum Zone {
    Base,
    Z1,
    Z2,
    Z3,
    Z4,
    Z5,
}

impl Zone {
    pub fn from_distance(d: i32) -> Zone {
        match d {
            ..=2 => Zone::Base,
            3..=20 => Zone::Z1,
            21..=45 => Zone::Z2,
            46..=75 => Zone::Z3,
            76..=110 => Zone::Z4,
            _ => Zone::Z5,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Zone::Base => "Base",
            Zone::Z1 => "Zone 1",
            Zone::Z2 => "Zone 2",
            Zone::Z3 => "Zone 3",
            Zone::Z4 => "Zone 4",
            Zone::Z5 => "Zone 5",
        }
    }
}

/// Index des tiles par position pour lookup O(1)
#[derive(Resource, Default)]
pub struct TileMap {
    pub tiles: HashMap<GridPos, Entity>,
}

/// Valeur totale d'un ensemble de ressources (somme valeur * quantité)
pub fn total_value(items: &HashMap<TileType, u32>) -> u64 {
    items.iter().map(|(t, qty)| t.value() * (*qty as u64)).sum()
}

/// Event envoyé quand une tile est minée
#[derive(Event)]
pub struct TileMinedEvent {
    pub pos: GridPos,
}

/// Graine de génération du monde (carte + gisements).
/// Surchargeable via la variable d'environnement `PIXELMINER_SEED`.
#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldSeed(pub u64);

impl WorldSeed {
    pub const ENV_VAR: &'static str = "PIXELMINER_SEED";

    /// Lit `PIXELMINER_SEED` si présente et valide, sinon tire une graine aléatoire.
    pub fn from_env_or_random() -> WorldSeed {
        let from_env = std::env::var(Self::ENV_VAR)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        WorldSeed(from_env.unwrap_or_else(rand::random))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_boundaries() {
        assert_eq!(Zone::from_distance(0), Zone::Base);
        assert_eq!(Zone::from_distance(2), Zone::Base);
        assert_eq!(Zone::from_distance(3), Zone::Z1);
        assert_eq!(Zone::from_distance(20), Zone::Z1);
        assert_eq!(Zone::from_distance(21), Zone::Z2);
        assert_eq!(Zone::from_distance(45), Zone::Z2);
        assert_eq!(Zone::from_distance(46), Zone::Z3);
        assert_eq!(Zone::from_distance(75), Zone::Z3);
        assert_eq!(Zone::from_distance(76), Zone::Z4);
        assert_eq!(Zone::from_distance(110), Zone::Z4);
        assert_eq!(Zone::from_distance(111), Zone::Z5);
        assert_eq!(Zone::from_distance(150), Zone::Z5);
    }

    #[test]
    fn from_world_rounds_to_nearest() {
        assert_eq!(GridPos::from_world(Vec2::ZERO), GridPos { x: 0, y: 0 });
        assert_eq!(GridPos::from_world(Vec2::new(7.9, -7.9)), GridPos { x: 0, y: 0 });
        assert_eq!(GridPos::from_world(Vec2::new(8.1, -8.1)), GridPos { x: 1, y: -1 });
        assert_eq!(GridPos::from_world(Vec2::new(32.0, -48.0)), GridPos { x: 2, y: -3 });
        let p = GridPos { x: -17, y: 42 };
        assert_eq!(GridPos::from_world(p.to_world()), p);
    }

    #[test]
    fn manhattan_distance() {
        let a = GridPos { x: 1, y: -2 };
        let b = GridPos { x: -3, y: 4 };
        assert_eq!(a.manhattan(&b), 10);
        assert_eq!(a.manhattan(&a), 0);
    }
}
