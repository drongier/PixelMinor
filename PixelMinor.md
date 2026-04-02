# PixelMiner — Game Design Document

## Vue d'ensemble

**Titre :** PixelMiner
**Genre :** Mining / Automatisation / Idle-Tycoon en vue de dessus
**Moteur :** Bevy (Rust)
**Style visuel :** Pixel art minimaliste (tiles 16×16)
**Plateforme :** PC (Desktop)
**Résolution :** 1920×1080 (Full HD)

PixelMiner est un jeu de minage et d'automatisation en vue top-down sur une grille plate. Le joueur commence seul, pioche en main, à creuser le terrain case par case. Plus il s'éloigne de sa base, plus les matériaux deviennent rares et précieux. Il transporte ses ressources jusqu'à une zone de décharge qui les vend automatiquement, puis réinvestit l'argent pour débloquer de meilleurs outils — et surtout, des machines et des bots qui automatisent progressivement chaque étape du processus. L'objectif ultime : passer d'un mineur solitaire à un empire minier entièrement automatisé.

---

## Boucle de gameplay principale

```
  ┌──────────────────────────────────────────────────────┐
  │                                                      │
  │   MINER  ──▶  CREUSER  ──▶  RÉCOLTER                │
  │     ▲                         │                      │
  │     │                         ▼                      │
  │   UPGRADE ◀── ACHETER ◀── VENDRE (auto)             │
  │     │                                                │
  │     ▼                                                │
  │   AUTOMATISER  ──▶  OBSERVER  ──▶  OPTIMISER         │
  │     (auto-mineurs, bots, convoyeurs)                 │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Phase manuelle (early game)
1. Le joueur déplace le mineur sur la grille avec ZQSD / flèches
2. Le mineur creuse en se déplaçant contre une tile (minage au contact)
3. Les matériaux extraits vont dans l'inventaire (capacité limitée)
4. Le joueur retourne à la zone de décharge → vente automatique au contact
5. L'argent permet d'acheter des upgrades dans la zone d'achat

### Phase automatisation (mid/late game)
6. Le joueur découvre des **gisements** cachés sous les tiles
7. Il place des **auto-mineurs** sur les gisements qui produisent passivement
8. Des **convoyeurs** transportent les ressources vers la base
9. Des **bots éclaireurs** trouvent de nouveaux gisements
10. Les convoyeurs peuvent se **fusionner** pour centraliser les flux

---

## Structure de la map

### Layout général

La map est une grille de **150×150** tiles, vue de dessus. La **base** est au centre de la map. Plus on s'éloigne de la base dans n'importe quelle direction, plus les matériaux sont rares et précieux. La rareté est calculée par la **distance de Manhattan** entre la tile et la base.

```
                        Zone 5 (très loin)
                   ┌─────────────────────────┐
                   │  Zone 4 (loin)          │
                   │  ┌───────────────────┐  │
                   │  │  Zone 3 (moyen)   │  │
                   │  │  ┌─────────────┐  │  │
                   │  │  │ Zone 2      │  │  │
                   │  │  │ ┌─────────┐ │  │  │
                   │  │  │ │ Zone 1  │ │  │  │
                   │  │  │ │ ┌─────┐ │ │  │  │
                   │  │  │ │ │ BASE│ │ │  │  │
                   │  │  │ │ │SHOP │ │ │  │  │
                   │  │  │ │ │SELL │ │ │  │  │
                   │  │  │ │ └─────┘ │ │  │  │
                   │  │  │ └─────────┘ │  │  │
                   │  │  └─────────────┘  │  │
                   │  └───────────────────┘  │
                   └─────────────────────────┘
```

### La base (centre de la map)

La base est une zone dégagée de rayon 2 (BASE_RADIUS) au centre, qui contient :
- **Zone d'achat (Shop)** : tile rose en haut (0, 40), touche E pour ouvrir le menu
- **Zone de décharge (Sell)** : tile verte en bas (0, -40), vente auto au contact

Le joueur spawn au centre (0, 0).

### Taille de la grille

- Taille : GRID_SIZE = 150 (150×150 tiles)
- Base au centre : position (0, 0)
- Chaque tile : 16×16 pixels
- Caméra centrée sur le joueur avec interpolation douce

### Zones de distance

La distance depuis la base détermine la zone :

| Zone   | Distance (tiles) | Description              |
|--------|-------------------|--------------------------|
| Base   | 0-2               | Zone dégagée, shop, sell |
| Zone 1 | 3-20              | Terre et pierre          |
| Zone 2 | 21-45             | Fer et cuivre            |
| Zone 3 | 46-75             | Argent et or             |
| Zone 4 | 76-110            | Diamant et rubis         |
| Zone 5 | 111+              | Mythril et obsidienne    |

### Types de tiles

| Tile        | Couleur RGB              | Dureté (HP) | Valeur |
|-------------|--------------------------|-------------|--------|
| Terre       | (0.55, 0.41, 0.08)       | 1           | 1 ¢    |
| Pierre      | (0.50, 0.50, 0.50)       | 2           | 2 ¢    |
| Charbon     | (0.20, 0.20, 0.20)       | 2           | 5 ¢    |
| Fer         | (0.72, 0.45, 0.20)       | 3           | 10 ¢   |
| Cuivre      | (0.80, 0.50, 0.20)       | 3           | 12 ¢   |
| Argent      | (0.75, 0.75, 0.75)       | 4           | 25 ¢   |
| Or          | (1.00, 0.84, 0.00)       | 5           | 50 ¢   |
| Diamant     | (0.00, 1.00, 1.00)       | 7           | 150 ¢  |
| Rubis       | (1.00, 0.00, 0.25)       | 7           | 200 ¢  |
| Mythril     | (0.48, 0.41, 0.93)       | 9           | 500 ¢  |
| Obsidienne  | (0.10, 0.04, 0.18)       | 10          | 750 ¢  |

Les tiles non-révélées sont affichées en couleur sombre (0.15, 0.12, 0.10). Elles se révèlent quand une tile adjacente est minée.

### Probabilités de spawn par zone

```
Zone 1 (distance: 3-20) :
  70% Terre, 20% Pierre, 8% Charbon, 2% Fer

Zone 2 (distance: 21-45) :
  30% Terre, 40% Pierre, 5% Charbon, 15% Fer, 10% Cuivre

Zone 3 (distance: 46-75) :
  50% Pierre, 20% Fer, 5% Cuivre, 15% Argent, 10% Or

Zone 4 (distance: 76-110) :
  40% Pierre, 20% Argent, 15% Or, 15% Diamant, 10% Rubis

Zone 5 (distance: 111+) :
  30% Pierre, 25% Obsidienne, 20% Mythril, 15% Diamant, 10% Rubis
```

---

## Gisements (Deposits)

### Concept

Les gisements sont des clusters de 2-4 tiles cachés sous les tiles normales, générés aléatoirement à la création de la map (~80 gisements). Ils se révèlent quand le joueur mine la tile au-dessus. Un gisement révélé affiche une couleur teintée selon son type.

### Types de gisements

Chaque type a une table de drop différente et apparaît dans certaines zones :

| Type     | Zones | Couleur     | Table de drop                                    |
|----------|-------|-------------|--------------------------------------------------|
| Ferreux  | 1, 2  | Rouille     | 70% Fer, 20% Charbon, 8% Cuivre, 2% Argent      |
| Précieux | 2, 3  | Doré        | 50% Argent, 30% Or, 15% Cuivre, 5% Diamant      |
| Gemmes   | 3, 4  | Cyan        | 40% Diamant, 35% Ruby, 20% Or, 5% Mythril       |
| Mythique | 4, 5  | Violet      | 45% Mythril, 30% Obsidienne, 15% Diamant, 10% Ruby |

### Affichage HUD

Quand le joueur marche sur un gisement révélé, le panneau stats affiche :
- Le nom du gisement
- La table de drop avec les pourcentages
- "[B] Poser un mineur" ou le stock du mineur si un mineur est déjà posé

---

## Le mineur (joueur)

### Déplacement et minage

- **Contrôles :** ZQSD ou flèches directionnelles
- **Minage :** au contact — le joueur se déplace contre une tile pour la miner
- **Dégâts :** `mining_power * delta_time` appliqué à chaque frame de contact
- **Récolte :** quand la tile est détruite, l'item est ajouté à l'inventaire (si pas plein)

### Stats du mineur (valeurs par défaut)

| Stat               | Défaut | Description                              |
|--------------------|--------|------------------------------------------|
| Puissance de minage | 5.0    | Dégâts par seconde à la tile             |
| Vitesse de marche   | 250.0  | Pixels par seconde                       |
| Capacité inventaire | 50     | Nombre max d'items transportables        |

---

## Contrôles

| Touche        | Action                                          |
|---------------|------------------------------------------------|
| ZQSD / Flèches | Déplacement                                   |
| E / Entrée    | Ouvrir/fermer le shop (dans la zone d'achat)   |
| Échap         | Fermer le shop                                  |
| B             | Poser un auto-mineur / Collecter un auto-mineur |
| G             | Lancer un bot éclaireur                         |
| T             | Commencer/terminer le traçage d'un convoyeur   |

---

## Zone de décharge (auto-sell)

- **Position :** (0, -40) en world coords
- **Trigger :** collision du mineur avec la zone
- **Action :** vide l'inventaire, ajoute la valeur totale au portefeuille
- **Visuel :** tile verte

---

## Zone d'achat (shop)

### Position et interaction

- **Position :** (0, 40) en world coords, tile rose
- **Interaction :** touche E quand le joueur est dans la zone

### Upgrades disponibles

#### [1] Pioche (Puissance de minage)

| Nom              | Puissance | Coût     |
|------------------|-----------|----------|
| Pioche en bois   | 1.0       | Départ   |
| Pioche en pierre | 2.0       | 50 ¢     |
| Pioche en fer    | 4.0       | 200 ¢    |
| Pioche en or     | 7.0       | 1 000 ¢  |
| Pioche diamant   | 12.0      | 5 000 ¢  |

#### [2] Sac (Capacité inventaire)

| Nom          | Capacité | Coût     |
|--------------|----------|----------|
| Poches       | 10       | Départ   |
| Petit sac    | 20       | 100 ¢    |
| Sac à dos    | 40       | 500 ¢    |
| Chariot      | 80       | 2 000 ¢  |
| Wagon        | 150      | 10 000 ¢ |

#### [3] Vitesse (Vitesse de marche)

La vitesse de base est 150.0 px/s, multipliée par le multiplicateur :

| Nom            | Multiplicateur | Coût     |
|----------------|---------------|----------|
| Normal         | ×1.0          | Départ   |
| Rapide         | ×1.5          | 300 ¢    |
| Très rapide    | ×2.0          | 1 500 ¢  |
| Frénétique     | ×3.0          | 8 000 ¢  |

#### [4] Auto-mineur

- **Coût :** 100 ¢ l'unité
- **Pas de limite d'achat**
- Voir section Auto-mineurs

#### [5] Bot éclaireur

- **Coût :** 50 ¢ l'unité
- **Pas de limite d'achat**
- Voir section Bots éclaireurs

---

## Auto-mineurs

### Placement

- Touche **B** à proximité (2.5 tiles) d'un gisement révélé sans mineur
- Consomme 1 auto-mineur de l'inventaire
- Visuel : petit carré orange (60% de la taille d'une tile) posé sur le gisement (z = -0.5)

### Production

- Produit **1 ressource toutes les 10 secondes** selon la drop table du gisement
- **Stock interne max : 50 ressources**
- S'arrête quand le stock est plein

### Collecte

- Touche **B** à proximité d'un mineur avec du stock
- Transfère les ressources dans l'inventaire du joueur (limité par la capacité)

### Priorité de la touche B

1. Si le joueur a des mineurs ET un gisement révélé sans mineur est à proximité → **placer**
2. Sinon si un mineur avec stock est à proximité → **collecter**
3. Sinon → rien

---

## Bots éclaireurs

### Déploiement

- Touche **G** pour lancer un bot depuis la position du joueur
- Le bot cherche le **gisement non-révélé le plus proche** (distance Manhattan)
- S'il n'y en a pas → rien ne se passe

### Comportement

- Entité visible : petit carré vert (50% de la taille d'une tile, z = 0.5)
- Se déplace tile par tile vers le gisement cible
- **Mine chaque tile sur son passage** (même puissance que le joueur au moment du lancement)
- Ne collecte rien, les tiles sont détruites
- Déclenche `TileMinedEvent` (révèle les voisins normalement)

### Pathfinding

- Simple : avance vers la cible en réduisant la distance Manhattan
- Choisit la direction (X ou Y) qui rapproche le plus de la cible
- Mine tout sur son passage, pas de contournement

### Arrivée

- Révèle le gisement cible
- Le bot **explose** (despawn)

---

## Convoyeurs (Tapis roulants)

### Traçage

- Touche **T** pour entrer en mode traçage
- Le joueur marche et pose des tapis le long de son chemin
- Chaque tile coûte **10 ¢** (déduit en temps réel)
- Re-**T** pour terminer le tracé
- Feedback : tiles semi-transparentes pendant le traçage, indicateur HUD ">> MODE TAPIS <<"

### Direction

- Tous les tapis d'un tracé vont **du premier point posé vers le dernier**
- Les ressources voyagent dans cette direction

### Connexion source

- Au moins un tile du tracé doit être **adjacent à un auto-mineur**
- Si aucun mineur n'est adjacent → tracé annulé et remboursé
- Le tapis aspire automatiquement le stock du mineur adjacent

### Destination (fin du tracé)

- **Zone de vente** (à proximité de la base) → ressources vendues automatiquement
- **Bout du tapis sans connexion** → ressources s'accumulent sans limite, récupérables avec **B**

### Transport

- Vitesse : **1 tile toutes les 5 secondes**
- Une ressource à la fois par slot du tapis
- Visuel : chaque tile du tapis prend la **couleur du minerai** qu'elle transporte, gris quand vide

### Fusion de tapis

- Pendant le traçage, si le joueur marche sur un tile qui a **déjà un tapis**, le tracé se termine automatiquement
- Le nouveau tapis déverse ses ressources dans le tapis existant au point de jonction
- Permet de connecter plusieurs mineurs vers un seul tapis central

### Messages d'erreur (HUD)

Si le tracé échoue :
- "Tapis trop court (min 2 tiles)"
- "Aucun mineur adjacent au tapis !"
- "Plus assez d'argent !"

---

## Interface utilisateur (HUD)

### Barre du haut

- **Argent** : affiché en jaune doré (taille 18)

### Panneau stats (haut gauche)

Affiché en permanence :
```
-- Stats --
Pos: (x, y)
Zone: Zone X (dist N)
Puissance: X
Vitesse: X
Sac: X/X
Argent: X c
Mineurs: X
Bots: X
```

Quand le joueur est sur un gisement :
```
-- Gisement --
Gisement ferreux
Drop:
  Fer : 70%
  Charbon : 20%
  Cuivre : 8%
  Argent : 2%
[B] Poser un mineur
```

En mode traçage :
```
>> MODE TAPIS <<
Tiles: X (cout: X c)
[T] Terminer
```

### Barre du bas

- **Inventaire** : liste des items avec quantités, format "Inventaire (X/X) : Terre x5   Fer x3"

---

## Architecture du code

### Structure des fichiers

```
src/
├── main.rs
├── world/              # Map, terrain, gisements
│   ├── mod.rs
│   ├── tiles.rs        # TileType, Tile, GridPos, constantes
│   ├── map.rs          # Génération de la map, révélation
│   └── deposits.rs     # DepositType, Deposit, spawn des gisements
├── player/             # Joueur
│   ├── mod.rs
│   └── player.rs       # Player, mouvement, minage au contact
├── machines/           # Automatisation
│   ├── mod.rs
│   ├── auto_miner.rs   # AutoMiner, MinerInventory, placement/collecte (B)
│   ├── scout_bot.rs    # ScoutBot, BotInventory, déploiement (G)
│   └── conveyor.rs     # ConveyorBelt, traçage (T), transport, fusion
├── economy/            # Argent, shop, vente
│   ├── mod.rs
│   ├── resources.rs    # Inventory, Wallet, PlayerStats, ShopOpen
│   ├── shop.rs         # ShopPlugin, upgrades, menu d'achat
│   └── sell.rs         # SellPlugin, zone de vente automatique
└── ui/                 # Interface
    ├── mod.rs
    └── hud.rs          # HudPlugin, affichage stats/inventaire/gisement
```

### Plugins Bevy

```
App
├── MapPlugin           # Génération et rendu de la grille
├── DepositPlugin       # Spawn et révélation des gisements
├── AutoMinerPlugin     # Auto-mineurs : placement, production, collecte
├── ScoutBotPlugin      # Bots éclaireurs : déploiement et pathfinding
├── ConveyorPlugin      # Convoyeurs : traçage, transport, fusion, vente
├── PlayerPlugin        # Mouvement et minage du joueur
├── SellPlugin          # Zone de vente automatique
├── ShopPlugin          # Menu d'achat et upgrades
└── HudPlugin           # Interface utilisateur
```

### Resources globales

| Resource       | Description                    |
|----------------|--------------------------------|
| Inventory      | Items collectés (HashMap)      |
| Wallet         | Argent du joueur               |
| PlayerStats    | Puissance, vitesse, capacité   |
| ShopOpen       | État du menu shop (bool)       |
| MinerInventory | Nombre d'auto-mineurs en stock |
| BotInventory   | Nombre de bots en stock        |
| ConveyorState  | État du mode traçage           |
| ConveyorBelts  | Liste de tous les tapis actifs |

---

## Progression et courbe de difficulté

### Early game — Le mineur solitaire (0-500 ¢)

Le joueur apprend les bases : creuser autour de la base, revenir vendre. Tout est manuel. Terre et pierre faciles. Premiers achats : pioche en pierre, petit sac. Le joueur ressent la répétitivité des allers-retours → motivation pour automatiser.

### Mid game — Découverte et automatisation (500-5 000 ¢)

Le joueur achète des bots éclaireurs pour découvrir les gisements, place des auto-mineurs dessus. Premiers convoyeurs vers la base. Moment "wow" : les ressources arrivent toutes seules.

### Late game — L'empire minier (5 000+ ¢)

Le joueur gère un réseau de mineurs et de convoyeurs fusionnés. Exploration des zones profondes pour des gisements mythiques. Optimisation des routes de convoyeurs.

---

## Roadmap de développement

### Phase 1 — Prototype jouable ✅

- [x] Setup Bevy avec fenêtre et rendu pixel-perfect
- [x] Grille de base avec tous les types de tiles (11 minerais)
- [x] Mineur : mouvement et minage au contact
- [x] Inventaire avec capacité limitée
- [x] Zone de décharge fonctionnelle (vente auto)
- [x] Zone d'achat avec upgrades (pioche, sac, vitesse)
- [x] HUD (argent, inventaire, stats, zone)
- [x] Caméra avec suivi fluide

### Phase 2 — Gisements et automatisation ✅

- [x] Gisements : 4 types, génération par clusters, révélation
- [x] Auto-mineurs : placement sur gisements, production passive, collecte
- [x] Bots éclaireurs : pathfinding, minage en chemin, révélation de gisement
- [x] Convoyeurs : traçage, transport visuel, vente auto, fusion

### Phase 3 — Polish et contenu (à venir)

- [ ] Sprites pixel art (remplacer les carrés colorés)
- [ ] Animations de minage et particules
- [ ] Sons 8-bit
- [ ] Sauvegarde / chargement
- [ ] Menu principal
- [ ] Feedback visuel amélioré (textes flottants "+XX ¢")

### Phase 4 — Contenu avancé (à venir)

- [ ] Nouveaux bâtiments (hub de collecte, usine de raffinage)
- [ ] Upgrades de machines (mineurs plus rapides, convoyeurs plus rapides)
- [ ] Mini-map
- [ ] Événements aléatoires
- [ ] Statistiques de production (¢/min)

---

## Dépendances

```toml
[dependencies]
bevy = "0.15"
rand = "0.8"
```
