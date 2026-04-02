use bevy::prelude::*;
use std::collections::HashMap;

use crate::world::tiles::TileType;

#[derive(Resource, Default)]
pub struct Inventory {
    pub items: HashMap<TileType, u32>,
}

#[derive(Resource, Default)]
pub struct Wallet {
    pub money: u64,
}

#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

#[derive(Resource)]
pub struct PlayerStats {
    pub mining_power: f32,
    pub walk_speed: f32,
    pub inventory_capacity: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            mining_power: 5.0,
            walk_speed: 250.0,
            inventory_capacity: 50,
        }
    }
}

impl Inventory {
    pub fn total_count(&self) -> u32 {
        self.items.values().sum()
    }
}
