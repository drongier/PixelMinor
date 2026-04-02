# Bot Éclaireur - Design Spec

## Achat

- Slot [5] Bot éclaireur — 50 coins dans le shop
- Stocké dans une resource `BotInventory { count: u32 }`
- Pas de limite d'achat

## Déploiement

- Touche G pour lancer un bot depuis la position du joueur
- Le bot cherche le gisement non-révélé le plus proche (distance Manhattan)
- S'il n'y en a pas, rien ne se passe

## Comportement

- Entité visible (petit carré vert)
- Se déplace tile par tile vers le gisement cible
- Mine chaque tile sur son passage (même vitesse que le joueur, dégâts basés sur HP de la tile)
- Ne collecte rien, les tiles sont détruites
- Déclenche TileMinedEvent pour chaque tile minée (révèle les voisins)

## Arrivée

- Quand le bot atteint la tile du gisement, le gisement est révélé
- Le bot explose (despawn)

## Pathfinding

- Simple : avance vers la cible en réduisant la distance Manhattan
- À chaque step, choisit la direction (haut/bas/gauche/droite) qui rapproche le plus de la cible
- Mine tout sur son passage, pas de contournement

## Structure technique

### Nouveau fichier : `scout_bot.rs`

- `ScoutBot` component : target (GridPos), mining_timer (Timer), current_tile_hp (f32)
- `BotInventory` resource : count (u32)
- `ScoutBotPlugin` : systèmes de déploiement et mouvement/minage
- Systèmes :
  - `deploy_bot` : touche G, trouve gisement non-révélé le plus proche, spawn bot
  - `bot_movement` : tick timer, mine la tile devant, avance quand tile détruite, despawn à l'arrivée

### Modifications existantes

- `shop.rs` : ajouter slot [5] pour acheter des bots à 50 coins
- `main.rs` : ajouter `ScoutBotPlugin`
- `hud.rs` : afficher le nombre de bots en inventaire
