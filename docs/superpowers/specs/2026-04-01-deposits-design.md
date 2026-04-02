# Gisements (Deposits) - Design Spec

## Concept

Des clusters de 2-4 tiles placés aléatoirement à la génération de la map, cachés sous les tiles minables normales. Ils se révèlent quand le joueur mine la tile au-dessus. Chaque gisement a un type avec une table de drop spécifique. Les auto-mineurs (futur) pourront être posés dessus.

## Types de gisements

| Type | Zones d'apparition | Table de drop |
|------|-------------------|---------------|
| Ferreux | 1, 2 | 70% Fer, 20% Charbon, 8% Cuivre, 2% Argent |
| Précieux | 2, 3 | 50% Argent, 30% Or, 15% Cuivre, 5% Diamant |
| Gemmes | 3, 4 | 40% Diamant, 35% Ruby, 20% Or, 5% Mythril |
| Mythique | 4, 5 | 45% Mythril, 30% Obsidienne, 15% Diamant, 10% Ruby |

## Génération

- ~15-20 gisements placés aléatoirement sur la map à la génération
- Chaque gisement = cluster de 2-4 tiles adjacentes (forme aléatoire par croissance depuis un point central)
- Le type dépend de la zone de profondeur du point central du cluster
- Les tiles de gisement existent sur un z-layer inférieur (z = -2.0) aux tiles normales (z = -1.0)
- Un gisement ne peut pas spawn dans la base (distance <= BASE_RADIUS)

## Révélation

- Quand une tile normale est minée et détruite, si une tile de gisement existe en dessous, elle devient visible
- Le système `reveal_on_mine` existant est étendu pour aussi révéler les deposits

## Rendu visuel

- Les tiles de gisement ont une couleur de fond sombre/neutre avec un marqueur visuel distinct
- Couleur de base : gris foncé légèrement teinté selon le type (pour indiquer visuellement le type une fois révélé)
- Marqueurs par type :
  - Ferreux : teinte rouille
  - Précieux : teinte dorée
  - Gemmes : teinte cyan
  - Mythique : teinte violette

## Structure technique

### Nouveau fichier : `deposits.rs`

- `DepositType` enum : Ferrous, Precious, Gems, Mythic
- `Deposit` component : deposit_type, revealed
- `DepositPlugin` : systèmes de spawn et révélation
- Méthodes sur `DepositType` : `color()`, `drop_table()`, `label()`

### Modifications existantes

- `map.rs` : après le spawn de la map, appeler le spawn des deposits
- `main.rs` : ajouter `DepositPlugin`

### Spawn des deposits

1. Choisir ~15-20 positions aléatoires sur la grille (hors base)
2. Pour chaque position, déterminer la zone et choisir un type de gisement compatible
3. Faire croître un cluster de 2-4 tiles adjacentes depuis ce point
4. Spawner les entités deposit (non révélées) sur z = -2.0

### Révélation des deposits

- Écouter `TileMinedEvent`
- Quand une tile est minée, chercher si un deposit existe à la même `GridPos`
- Si oui, le marquer comme révélé et mettre à jour sa couleur
