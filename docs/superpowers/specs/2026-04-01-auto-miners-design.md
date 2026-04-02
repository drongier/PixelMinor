# Auto-mineurs - Design Spec

## Achat

- Nouveau slot au shop : [4] Auto-mineur — 100 coins
- Pas de limite d'achat, pas de tiers
- Stocké dans une ressource `MinerInventory { count: u32 }`

## Placement

- Touche B à proximité (2 tiles manhattan distance) d'un gisement révélé sans mineur
- Consomme 1 mineur de l'inventaire, spawn un auto-mineur sur la tile du gisement
- Un seul mineur par tile de gisement
- Visuellement : petit carré coloré distinct posé sur le gisement

## Production

- 1 ressource toutes les 10s selon la drop_table du DepositType du gisement
- Stock interne max : 50 ressources
- Composant `AutoMiner { storage: HashMap<TileType, u32>, timer: Timer }`

## Collecte

- Touche B à proximité d'un mineur avec du stock
- Transfert dans l'inventaire du joueur (limité par la capacité)

## Logique touche B (priorité)

1. Si le joueur a des mineurs ET un gisement révélé sans mineur est à proximité → placer
2. Sinon si un mineur avec stock est à proximité → collecter
3. Sinon → rien

## Structure technique

### Nouveau fichier : `auto_miner.rs`

- `AutoMiner` component : storage (HashMap<TileType, u32>), timer (Timer), deposit_type (DepositType)
- `MinerInventory` resource : count (u32)
- `AutoMinerPlugin` : systèmes de placement, production, collecte
- Systèmes :
  - `handle_b_key` : logique placement/collecte
  - `auto_miner_production` : tick les timers et produit des ressources
  - `spawn_auto_miner_visual` : spawn le sprite du mineur

### Modifications existantes

- `shop.rs` : ajouter slot [4] pour acheter des auto-mineurs
- `main.rs` : ajouter `AutoMinerPlugin`, insérer `MinerInventory` resource
- `hud.rs` : afficher le nombre de mineurs en inventaire
