# PixelMiner

A mining and automation game in top down view, built with **Bevy** (Rust).

Start as a lone miner with a pickaxe, digging tile by tile. The further you go from your base, the rarer and more valuable the materials become. Sell your resources, upgrade your tools, and progressively build an empire of automated machines that mine, transport, and sell for you.

---

## Gameplay

### Core Loop

```
  MINE  ──▶  DIG  ──▶  COLLECT
    ▲                     │
    │                     ▼
  UPGRADE ◀── BUY ◀── SELL (auto)
    │
    ▼
  AUTOMATE  ──▶  OBSERVE  ──▶  OPTIMIZE
```

### Manual Phase (Early Game)

1. Move your miner on the grid with ZQSD or arrow keys
2. Dig by walking into tiles (contact mining)
3. Collected materials go into your inventory (limited capacity)
4. Walk back to the sell zone at the base for automatic selling
5. Spend coins in the shop to buy upgrades

### Automation Phase (Mid/Late Game)

6. Discover **deposits** hidden beneath tiles
7. Place **auto miners** on deposits for passive production
8. Build **conveyor belts** to transport resources to the base
9. Deploy **scout bots** to find new deposits automatically
10. Merge conveyors to centralize resource flows

---

## The Map

A **150x150** tile grid with the base at the center. Distance from the base determines the zone and the types of ores that spawn.

| Zone   | Distance | Resources                    |
|--------|----------|------------------------------|
| Base   | 0 to 2   | Empty area, shop, sell zone  |
| Zone 1 | 3 to 20  | Dirt, Stone, Coal, Iron      |
| Zone 2 | 21 to 45 | + Copper                     |
| Zone 3 | 46 to 75 | + Silver, Gold               |
| Zone 4 | 76 to 110| + Diamond, Ruby              |
| Zone 5 | 111+     | + Mythril, Obsidian          |

### Ores

| Ore        | Hardness | Value   |
|------------|----------|---------|
| Dirt       | 1        | 1 coin  |
| Stone      | 2        | 2 coins |
| Coal       | 2        | 5 coins |
| Iron       | 3        | 10 coins|
| Copper     | 3        | 12 coins|
| Silver     | 4        | 25 coins|
| Gold       | 5        | 50 coins|
| Diamond    | 7        | 150 coins|
| Ruby       | 7        | 200 coins|
| Mythril    | 9        | 500 coins|
| Obsidian   | 10       | 750 coins|

Tiles are hidden until a neighboring tile is mined, creating a fog of war effect.

---

## Deposits

Clusters of 2 to 4 tiles hidden beneath the normal terrain, generated randomly across the map (~80 deposits). They are revealed when the tile above is mined.

Each deposit has a type with a specific drop table:

| Type     | Zones | Main drops                                      |
|----------|-------|-------------------------------------------------|
| Ferrous  | 1, 2  | 70% Iron, 20% Coal, 8% Copper, 2% Silver        |
| Precious | 2, 3  | 50% Silver, 30% Gold, 15% Copper, 5% Diamond    |
| Gems     | 3, 4  | 40% Diamond, 35% Ruby, 20% Gold, 5% Mythril     |
| Mythic   | 4, 5  | 45% Mythril, 30% Obsidian, 15% Diamond, 10% Ruby|

---

## Controls

| Key            | Action                                        |
|----------------|-----------------------------------------------|
| ZQSD / Arrows  | Move                                          |
| E / Enter      | Open or close the shop (in the shop zone)     |
| Escape         | Close the shop                                |
| B              | Place an auto miner or collect from one       |
| G              | Launch a scout bot                            |
| T              | Start or finish conveyor belt tracing         |

---

## Shop

Open the shop with **E** when standing in the shop zone (pink tile near the base).

| Slot | Item           | Cost       | Description                        |
|------|----------------|------------|------------------------------------|
| [1]  | Pickaxe        | 50 to 5000 | Increases mining power (5 tiers)   |
| [2]  | Bag            | 100 to 10000| Increases inventory capacity (5 tiers)|
| [3]  | Speed          | 300 to 8000| Increases walk speed (4 tiers)     |
| [4]  | Auto Miner     | 100 each   | Place on deposits for auto mining  |
| [5]  | Scout Bot      | 50 each    | Finds the nearest hidden deposit   |

---

## Auto Miners

- Press **B** near a revealed deposit to place one
- Produces **1 resource every 10 seconds** based on the deposit's drop table
- Internal storage of **50 resources** (stops when full)
- Press **B** near a miner to collect its stored resources

## Scout Bots

- Press **G** to launch from your position
- The bot moves toward the **nearest hidden deposit**
- It **mines every tile in its path** (same power as the player)
- On arrival, it **reveals the deposit and self destructs**

## Conveyor Belts

- Press **T** to start tracing, walk to draw the path, press **T** again to finish
- Each tile costs **10 coins**
- Must pass near an auto miner to connect (otherwise refunded)
- Resources travel at **1 tile per 5 seconds**
- Each belt tile shows the **color of the ore** it is carrying
- If the end of the belt is near the sell zone, resources are **sold automatically**
- Otherwise, resources accumulate at the end and can be collected with **B**
- Walking onto an existing belt during tracing **merges** the two belts automatically

---

## HUD

### Stats Panel (Top Left)

Shows position, zone, mining power, walk speed, inventory, coins, miners and bots in stock. When standing on a deposit, it also displays the deposit type and drop table.

### Top Bar

Displays current coin balance in gold.

### Bottom Bar

Shows inventory contents with item names and quantities.

### Tracing Mode

When placing conveyors, the HUD shows the number of tiles placed and total cost.

---

## Project Structure

```
src/
├── main.rs
├── world/              # Map, terrain, deposits
│   ├── tiles.rs        # Tile types, grid positions, constants
│   ├── map.rs          # Map generation and tile reveal
│   └── deposits.rs     # Deposit types, generation, reveal
├── player/             # Player character
│   └── player.rs       # Movement and contact mining
├── machines/           # Automation systems
│   ├── auto_miner.rs   # Auto miners: placement, production, collection
│   ├── scout_bot.rs    # Scout bots: pathfinding and mining
│   └── conveyor.rs     # Conveyors: tracing, transport, merging
├── economy/            # Money and trading
│   ├── resources.rs    # Inventory, Wallet, PlayerStats
│   ├── shop.rs         # Shop menu and upgrades
│   └── sell.rs         # Automatic sell zone
└── ui/                 # User interface
    └── hud.rs          # Stats panel, inventory bar, deposit info
```

---

## Building and Running

```bash
cd pixel_miner
cargo run
```

### Dependencies

- [Bevy](https://bevyengine.org/) 0.15
- rand 0.8

---

## Roadmap

### Done

- Full tile system with 11 ore types across 5 zones
- Player movement and contact mining
- Inventory, sell zone, shop with upgrades
- Deposits: 4 types, cluster generation, reveal on mine
- Auto miners: passive production on deposits
- Scout bots: pathfinding, mining, deposit reveal
- Conveyor belts: tracing, visual transport, auto sell, merging
- HUD with stats, inventory, deposit info, tracing mode

### Planned

- Pixel art sprites (replace colored squares)
- Mining animations and particles
- Sound effects
- Save and load system
- Main menu
- New buildings (collection hub, refinery)
- Machine upgrades
- Minimap
- Production statistics
