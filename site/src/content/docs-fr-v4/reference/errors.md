---
title: Codes d'erreur
description: "R\u00e9f\u00e9rence des codes d'erreur FerrFlow avec causes et solutions."
---

Quand FerrFlow rencontre une erreur, il affiche un code comme `error[E2001]` avec un lien vers cette page. Utilisez le code pour trouver la cause et la solution.

## Erreurs de configuration

### E1001 : Fichier de config introuvable

<span id="e1001"></span>

Le fichier de config indiqu\u00e9 via `--config` n'existe pas.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow init</code> pour cr\u00e9er un fichier de config, ou v\u00e9rifiez le chemin.</p>
</div></aside>

### E1002 : \u00c9chec du parsing ferrflow.json

<span id="e1002"></span>

Le fichier `ferrflow.json` contient du JSON invalide.

### E1003 : \u00c9chec du parsing ferrflow.json5

<span id="e1003"></span>

Le fichier `ferrflow.json5` contient du JSON5 invalide.

### E1004 : \u00c9chec du parsing ferrflow.toml

<span id="e1004"></span>

Le fichier `ferrflow.toml` contient du TOML invalide.

### E1005 : Erreur de s\u00e9rialisation TOML

<span id="e1005"></span>

Erreur interne lors de l'\u00e9criture TOML.

### E1006 : \u00c9chec du parsing .ferrflow

<span id="e1006"></span>

Le fichier `.ferrflow` contient du JSON invalide.

### E1007 : Erreur de s\u00e9rialisation .ferrflow

<span id="e1007"></span>

Erreur interne lors de l'\u00e9criture du dotfile.

### E1008 : R\u00e9solution de chemin impossible

<span id="e1008"></span>

Un chemin dans la config n'a pas pu \u00eatre r\u00e9solu en chemin absolu.

### E1009 : \u00c9criture du loader temporaire impossible

<span id="e1009"></span>

Impossible d'\u00e9crire le loader JS/TS temporaire.

### E1010 : Impossible d'ex\u00e9cuter tsx

<span id="e1010"></span>

Le runtime `tsx` est introuvable pour les configs `.ts`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Installez tsx : <code>npm install -g tsx</code>, ou utilisez un format JSON/TOML.</p>
</div></aside>

### E1011 : Impossible d'ex\u00e9cuter node

<span id="e1011"></span>

Le runtime `node` est introuvable pour les configs `.js`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Installez Node.js ou utilisez un format JSON/TOML.</p>
</div></aside>

### E1012 : \u00c9valuation de la config \u00e9chou\u00e9e

<span id="e1012"></span>

Le fichier JS/TS a lev\u00e9 une erreur lors de l'\u00e9valuation.

### E1013 : Sortie de config invalide

<span id="e1013"></span>

Le fichier JS/TS a produit une sortie non UTF-8.

### E1014 : JSON invalide depuis la config

<span id="e1014"></span>

Le fichier JS/TS n'a pas produit de JSON valide.

### E1015 : Lecture du fichier impossible

<span id="e1015"></span>

Le fichier de config existe mais ne peut pas \u00eatre lu.

### E1016 : Plusieurs fichiers de config

<span id="e1016"></span>

Plusieurs fichiers de config trouv\u00e9s dans le r\u00e9pertoire.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Gardez un seul fichier de config.</p>
</div></aside>

### E1017 : Fichier d\u00e9j\u00e0 existant

<span id="e1017"></span>

`ferrflow init` lanc\u00e9 alors qu'un fichier de config existe d\u00e9j\u00e0.

## Erreurs de validation

### E1100 : Spec de repo invalide

<span id="e1100"></span>

L'argument `--repo` ne correspond pas au format attendu `owner/repo`.

### E1101 : Erreur API GitHub

<span id="e1101"></span>

L'API GitHub a retourn\u00e9 une erreur lors de la validation distante.

### E1102 : Erreur API GitLab

<span id="e1102"></span>

L'API GitLab a retourn\u00e9 une erreur lors de la validation distante.

### E1103 : UTF-8 invalide

<span id="e1103"></span>

Le fichier de config distant contient un encodage UTF-8 invalide.

### E1104 : Parsing de la config distante \u00e9chou\u00e9

<span id="e1104"></span>

Le fichier de config distant n'a pas pu \u00eatre pars\u00e9.

### E1105 : Fichier de config distant introuvable

<span id="e1105"></span>

Le chemin sp\u00e9cifi\u00e9 n'existe pas dans le d\u00e9p\u00f4t distant.

### E1106 : Aucun fichier de config trouv\u00e9

<span id="e1106"></span>

Aucun fichier de config FerrFlow dans le d\u00e9p\u00f4t distant.

### E1107 : --ref n\u00e9cessite --repo

<span id="e1107"></span>

Le flag `--ref` a \u00e9t\u00e9 utilis\u00e9 sans `--repo`.

## Op\u00e9rations Git

### E2001 : Pas un d\u00e9p\u00f4t git

<span id="e2001"></span>

Le r\u00e9pertoire courant n'est pas dans un d\u00e9p\u00f4t git.

### E2002 : D\u00e9p\u00f4t bare non support\u00e9

<span id="e2002"></span>

FerrFlow ne supporte pas les d\u00e9p\u00f4ts git bare.

### E2003 : Tag existant

<span id="e2003"></span>

Le tag que FerrFlow veut cr\u00e9er existe d\u00e9j\u00e0.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Supprimez le tag existant ou utilisez <code>--force</code>.</p>
</div></aside>

### E2004 : Push de branche \u00e9chou\u00e9

<span id="e2004"></span>

Impossible de push la branche de release.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>V\u00e9rifiez vos droits de push et les r\u00e8gles de protection.</p>
</div></aside>

### E2005 : Push rejet\u00e9

<span id="e2005"></span>

Le remote a rejet\u00e9 le push.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Pullez les derniers changements et r\u00e9essayez.</p>
</div></aside>

### E2006 : Push des tags \u00e9chou\u00e9

<span id="e2006"></span>

Impossible de push les tags vers le remote.

### E2007 : Push des tags flottants \u00e9chou\u00e9

<span id="e2007"></span>

Impossible de force-push les tags flottants.

### E2008 : Remote introuvable

<span id="e2008"></span>

Le remote git configur\u00e9 n'existe pas.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>V\u00e9rifiez <code>git remote -v</code> et le champ <code>remote</code> de votre config.</p>
</div></aside>

### E2009 : V\u00e9rification post-push \u00e9chou\u00e9e

<span id="e2009"></span>

Le commit de release n'a pas pu \u00eatre v\u00e9rifi\u00e9 sur la branche distante.

### E2010 : Branche distante introuvable

<span id="e2010"></span>

La branche distante n'a pas \u00e9t\u00e9 trouv\u00e9e apr\u00e8s le push.

## API GitHub

### E3001 : Cr\u00e9ation de release \u00e9chou\u00e9e

<span id="e3001"></span>

L'API GitHub Releases a retourn\u00e9 une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>V\u00e9rifiez que <code>GITHUB_TOKEN</code> a la permission <code>contents: write</code>.</p>
</div></aside>

### E3002 to E3010 : Erreurs API GitHub

<span id="e3002"></span>

Erreurs lors d'op\u00e9rations sur l'API GitHub (releases, PR, auto-merge, GraphQL).

## API GitLab

### E3101 : Cr\u00e9ation de release \u00e9chou\u00e9e

<span id="e3101"></span>

L'API GitLab Releases a retourn\u00e9 une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>V\u00e9rifiez que le token CI a les acc\u00e8s API n\u00e9cessaires.</p>
</div></aside>

### E3102 to E3105 : Erreurs API GitLab

<span id="e3102"></span>

Erreurs lors d'op\u00e9rations sur l'API GitLab (releases, MR, merge).

## Fichiers de version

Les erreurs E4xxx concernent la lecture, l'\u00e9criture et le parsing des fichiers de version :

| Plage          | Format                            |
| -------------- | --------------------------------- |
| E4101 to E4105 | TOML (Cargo.toml, pyproject.toml) |
| E4201 to E4205 | JSON (package.json)               |
| E4301 to E4304 | Helm / YAML (Chart.yaml)          |
| E4401 to E4413 | XML / CSProj                      |
| E4501 to E4504 | Gradle                            |
| E4601 to E4603 | Go mod                            |
| E4701 to E4704 | Texte (VERSION, VERSION.txt)      |

Erreurs courantes : lecture impossible, syntaxe invalide, champ `version` manquant, \u00e9criture impossible, UTF-8 invalide.

## Pr\u00e9-release

### E5001 : Nom de channel vide

<span id="e5001"></span>

Le nom du channel de pr\u00e9-release est vide.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Sp\u00e9cifiez un nom : <code>--channel beta</code></p>
</div></aside>

### E5002 : Nom de channel invalide

<span id="e5002"></span>

Seuls les alphanum\u00e9riques et tirets sont accept\u00e9s.

## Versioning

### E5010 : Semver invalide

<span id="e5010"></span>

La version actuelle n'est pas un semver valide.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Format attendu : <code>MAJEUR.MINEUR.PATCH</code>.</p>
</div></aside>

## Hooks

### E6001 : Hook \u00e9chou\u00e9

<span id="e6001"></span>

Un hook a \u00e9chou\u00e9 avec `on_failure: "abort"`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>V\u00e9rifiez la commande du hook, ou mettez <code>on_failure: &quot;continue&quot;</code>.</p>
</div></aside>

## Query

### E7001 : Aucun package configur\u00e9

<span id="e7001"></span>

Aucun package dans le fichier de config.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow init</code> ou ajoutez des packages manuellement.</p>
</div></aside>

### E7002 : Package introuvable

<span id="e7002"></span>

Le nom de package sp\u00e9cifi\u00e9 n'existe pas dans la config.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow version</code> pour lister les packages.</p>
</div></aside>

## Monorepo

### E8001 : Package introuvable dans la config

<span id="e8001"></span>

Un package r\u00e9f\u00e9renc\u00e9 pendant la release n'a pas \u00e9t\u00e9 trouv\u00e9.

### E8002 : Tag flottant r\u00e9gressif

<span id="e8002"></span>

Un tag flottant serait d\u00e9plac\u00e9 vers une version plus ancienne.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Utilisez <code>--force</code> pour ignorer la v\u00e9rification.</p>
</div></aside>
