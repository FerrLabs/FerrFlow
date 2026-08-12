---
title: Télémétrie
description: FerrFlow ne collecte plus de télémétrie. Ce que les anciennes versions envoyaient, et comment la désactiver sur ces versions.
---

**FerrFlow ne collecte pas de télémétrie.** À partir de la v5.33, le CLI n'émet aucune requête réseau de son propre chef, en dehors des opérations git et forge que vous demandez explicitement. Il n'y a rien à désactiver, aucune variable d'environnement à poser, aucune donnée dont s'inquiéter.

## Si vous utilisez une version antérieure à la v5.33

Les versions jusqu'à la v5.32 envoyaient des événements d'usage anonymes (type de commande, nombre de commits, hash SHA-256 de l'URL du remote git) vers `api.ferrflow.com`. Aucun code source, nom de fichier, message de commit, profil par IP ni donnée personnelle n'a jamais été collecté. Sur ces versions, vous pouvez désactiver la télémétrie avec :

```bash
export FERRFLOW_TELEMETRY=0
# ou la convention inter-outils
export DO_NOT_TRACK=1
```

ou par dépôt dans `ferrflow.json` :

```json
{ "workspace": { "anonymous_telemetry": false } }
```

## Pourquoi elle a été retirée

La télémétrie ajoutait une latence mesurable à chaque commande pour les utilisateurs de monorepos, et le signal utile qu'elle fournissait existe sans elle : les diagnostics d'erreur pointent vers des pages de documentation par code, et l'adoption se lit dans les téléchargements de releases. La retirer était plus simple et plus honnête que la réparer. La clé de configuration `anonymous_telemetry` reste acceptée (et ignorée) pour que les configurations existantes restent valides.
