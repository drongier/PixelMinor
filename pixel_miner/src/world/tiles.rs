use bevy::prelude::*;

pub const TILE_SIZE: f32 = 16.0;
pub const GRID_SIZE: i32 = 150;
pub const BASE_RADIUS: i32 = 2;

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
    pub fn distance_from_center(&self) -> i32 {
        self.x.abs() + self.y.abs()
    }

    pub fn to_world(&self) -> Vec2 {
        Vec2::new(self.x as f32 * TILE_SIZE, self.y as f32 * TILE_SIZE)
    }

    pub fn neighbors(&self) -> [GridPos; 4] {
        [
            GridPos { x: self.x,     y: self.y + 1 },
            GridPos { x: self.x,     y: self.y - 1 },
            GridPos { x: self.x - 1, y: self.y     },
            GridPos { x: self.x + 1, y: self.y     },
        ]
    }
}

/// Event envoyé quand une tile est minée
#[derive(Event)]
pub struct TileMinedEvent {
    pub pos: GridPos,
}
