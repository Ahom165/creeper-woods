# Creeper Woods — recréation de Minecraft Dungeons en Rust + wgpu

Recréation **libre et from scratch** de la mission d'introduction **« Creeper Woods »**
de *Minecraft Dungeons* (Mojang, 2020) : moteur maison en **Rust** avec **wgpu**,
HUD **français**, textures **100 % procédurales** remplaçables par les textures du jeu,
props remplaçables par des modèles `.obj`.

> Aucun asset original n'est inclus ni distribué : le jeu embarque uniquement du code
> original et des textures générées par programme. Les vraies textures se greffent
> via le dossier `assets/textures/` (voir plus bas).

## Lancer le jeu

- **Windows** : `Creeper-Woods-Windows.exe` (aucune dépendance, GPU compatible Vulkan/DX12).
- **Linux** : `./Creeper-Woods-Linux` (X11, Vulkan ou GL).
- Depuis les sources : `cargo run --release`.

Mode capture headless (tests/CI, sans interaction) :

```sh
creeper_woods --capture captures/   # écrit 01_titre.png … 06_pause.png
MCD_DEBUG=1 creeper_woods           # affiche les avertissements wgpu
```

## Commandes (défauts PC — scancodes physiques, donc ZQSD sur AZERTY)

| Action | Touche |
| --- | --- |
| Se déplacer | ZQSD / WASD |
| Mêlée (combo 1-2-3) | Clic gauche (maintenir) |
| Arc (charger → relâcher) | Clic droit |
| Roulade (i-frames 0,42 s) | Espace |
| Potion (75 % des PV, 30 s) | E |
| Artefacts 1 / 2 / 3 | 1 / 2 / 3 |
| Interagir (villageois, coffres, bouton…) | F |
| Carte | M |
| Inventaire (acheter les enchantements) | I |
| Pause | Échap |

## Ce qui est fidèle au jeu (sources : wiki + critiques + avis joueurs)

- **Mission** : libérer le villageois → retrouver la caravane → libérer 5 villageois
  (vagues à chaque cage) → tombe runique → **Nécromancien** → fuite du temple.
- **10 zones** : lac aux têtes de pierre, bois, auberge, grotte aux araignées (toiles,
  marchand capturé), salle des creepers (têtes sculptées), rivière à deux ponts,
  village encerclé, cimetière (crypte secrète : plaque de pression + parchemin),
  tombe antique (bouton → porte runique), pont de lave et temple.
- **Progression** : 100 PV, **3 vies par mission** sans autre pénalité, points de
  contrôle de zone, XP → **1 point d'enchantement par niveau** (Tranchant / Puissance /
  Robustesse), émeraudes, flèches (charge 25 → 60), **âmes plafonnées à 300**.
- **Combo de mêlée mémorisé pendant la pause** (relevé chez les joueurs, g36Chris) ;
  le 3ᵉ coup frappe plus fort et plus large.
- **Ennemis** : zombie, squelette (archer), creeper (mèche + explosion), araignée,
  vindicateur, **enchanteur** (aura violette → ennemis **enchantés**, PV/dégâts/XP ×).
- **Nécromancien** : 600 PV, invocations **plafonnées à 7 zombies**, salves d'orbes
  repoussantes, téléportations, paliers à 75/50/25 %, barre de boss.
- **Artéfacts** : Cor du vent (repousse), Flèches d'artifice (3 fusées à tête
  chercheuse), Guérison d'âme (50 âmes).
- **Direction artistique** : caméra 3/4 surélevée, brouillard de nuit, vignette,
  éclairage dramatique (champignons lumineux, braséros, lave émissive), dégâts
  flottants, flash rouge, pouls PV bas.
- **Écrans français** : titre, pause (avec la note sur le combo mémorisé), mort,
  échec, victoire (temps, ennemis, villageois, secrets, émeraudes, niveau).

## Utiliser les vraies textures du jeu

Le jeu charge au démarrage chaque PNG présent dans `assets/textures/` et le plaque
sur la tuile correspondante (16×16, échantillonnage nearest). Sans PNG, la tuile
procédurale d'origine est utilisée — le jeu reste jouable et légal sans assets.

Noms de fichiers reconnus :

`grass`, `dirt`, `path`, `stone`, `moss_stone`, `planks`, `water`, `lava`, `bridge`,
`stone_wall`, `house_wall`, `window`, `roof`, `leaves`, `trunk`, `tombstone`, `cage`,
`rune_gate`, `pressure_plate`, `hero`, `zombie`, `skeleton`, `creeper`, `spider`,
`vindicator`, `enchanter`, `necromancer`, `villager`, `merchant` (`.png`).

**Extraction** (si vous possédez le jeu) :

1. Récupérez *Minecraft Dungeons Extractor* (outil communautaire sur GitHub).
2. Pointez-le vers votre installation du jeu et exportez les textures `.png`.
3. Renommez-les selon la liste ci-dessus et placez-les dans `assets/textures/`.
4. Lancez le jeu : les overrides sont appliqués (les erreurs de lecture retombent
   proprement sur la tuile procédurale).

## Utiliser des modèles 3D pour les props

Chaque type de prop accepte un remplacement `assets/models/<nom>.obj` (optionnellement
`<nom>.png` comme texture dédiée) :

`tree1`, `tree2`, `rock`, `mushroom`, `house`, `tombstone`, `brazier`, `statue`,
`caravan`, `portal`.

Conventions : Wavefront OBJ, +Y vers le haut, origine **au sol**, unités en mètres,
faces triangulées (éventail), indices négatifs non pris en charge ; sans `vt`/`vn`
les UV/normales sont déduites. Le rayon de collision est déduit de l'encombrement
du modèle. Sans `.obj`, le prop garde ses cubes procéduraux.

## Architecture / performances

- `src/gfx/` : device wgpu, 6 pipelines (terrain, entités instanciées, billboards
  alpha/additifs, interface, blit), cible composite + captures PNG.
- `src/level/` : génération déterministe (seed fixe), maillage par **chunks 16×16**
  avec **culling frustum**, props pré-calculés indexés par grille.
- `src/game/` : joueur, IA (7 profils), projectiles, ramassables, particules,
  objectifs, boss.
- Rendu **instancié** (un seul draw pour toutes les entités cubes), lumières :
  16 sources les plus proches + lumière du héros, éclairage calculé au vertex.
- `cargo clippy --all-targets -- -D warnings` : **zéro avertissement**.

## Compilation

```sh
cargo build --release          # binaire local
cargo clippy --all-targets     # lint
```

La CI (`.github/workflows/build.yml`) compile Windows `.exe` + Linux à chaque push,
publie les artefacts et met à jour la release `v0.1.0`.

## Limites connues

- Pas d'audio (mélodie chiptune prévue dans une itération ultérieure).
- Armure fixe (réduction 15 %) — pas d'objets d'armure à équiper.
- Le marchand délivré offre une récompense unique (pas de boutique complète) ;
  les émeraudes comptent dans le score final.
- Un seul biome (Creeper Woods) — la mission complète, pas les autres niveaux.

## Licence

Code sous licence MIT (`LICENSE`). Recréation à but pédagogique, non affiliée à
Mojang/Microsoft — les noms *Minecraft* et *Minecraft Dungeons* appartiennent à
leurs propriétaires respectifs.
