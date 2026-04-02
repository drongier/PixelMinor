# Tapis roulants (Conveyor Belts) - Design Spec

## Pose

- Touche T pour entrer/sortir du mode traçage
- En mode traçage, le joueur marche et pose des tapis le long de son chemin
- Chaque tile de tapis coûte 10 coins (déduit en temps réel)
- Si plus assez d'argent, le traçage s'arrête automatiquement
- Re-T pour terminer le tracé

## Direction

- Tous les tapis d'un tracé pointent du premier point posé vers le dernier
- Au moment du re-T (fin du tracé), la direction de chaque tile est calculée
- Les ressources voyagent du début (source) vers la fin (destination)

## Connexion source

- Le début du tracé doit être adjacent à un auto-mineur
- Le tapis aspire automatiquement le stock du mineur adjacent toutes les 5 secondes

## Destination (fin du tracé)

- Base/zone de vente : ressources vendues automatiquement (ajoutées au wallet)
- Bout du tapis sans connexion : ressources s'accumulent sans limite, le joueur récupère avec B

## Transport

- Vitesse : 1 tile toutes les 5 secondes
- Une ressource à la fois sur le tapis, la suivante part quand la précédente a avancé d'une tile
- Le tapis change de couleur quand il transporte (indicateur visuel simple)

## Visuel

- Couleur de base : gris métallique
- Couleur en transport : gris plus clair / légèrement bleuté
- Pas de rendu d'items individuels sur le tapis

## Structure technique

### Nouveau fichier : `conveyor.rs`

- `ConveyorTile` component : belt_id (u32), index (usize dans le tracé), direction
- `ConveyorBelt` resource/component : liste ordonnée des GridPos du tracé, storage au bout (HashMap<TileType, u32>)
- `ConveyorState` resource : mode traçage actif/inactif, tracé en cours, belt_id counter
- `ConveyorPlugin` : systèmes de traçage, transport, aspiration, livraison

### Systèmes

- `toggle_tracing` : T pour entrer/sortir du mode traçage
- `trace_path` : pendant le traçage, enregistre les positions du joueur et pose les tiles visuelles, déduit 10 coins par tile
- `finalize_belt` : au re-T, calcule les directions, vérifie la connexion source (adjacent à un mineur)
- `conveyor_pull` : aspire les ressources du mineur adjacent au début du tapis
- `conveyor_transport` : déplace les ressources tile par tile (timer 5s)
- `conveyor_deliver` : à la fin du tapis, vend (si base) ou stocke
- `collect_conveyor_end` : touche B à proximité du bout d'un tapis pour récupérer le stock

### Données de transport

- Chaque belt a un Vec de "slots" (Option<TileType>) correspondant à chaque tile du tracé
- Le transport tick toutes les 5s et shift les slots d'un cran vers la fin
- Le slot 0 aspire depuis le mineur si vide

### Modifications existantes

- `main.rs` : ajouter `ConveyorPlugin`
- `auto_miner.rs` : rendre le storage accessible publiquement (déjà pub)
- `sell.rs` : potentiellement réutiliser la logique de vente
