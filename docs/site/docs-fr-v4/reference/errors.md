---
title: Codes d'erreur
description: "Référence des codes d'erreur FerrFlow avec causes et solutions."
---

Quand FerrFlow rencontre une erreur, il affiche un code comme `error[E2001]` avec un lien vers cette page. Utilisez le code pour trouver la cause et la solution.

## Erreurs de configuration

### E1001 : Fichier de config introuvable

<span id="e1001"></span>

Le fichier de config indiqué via `--config` n'existe pas.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow init</code> pour créer un fichier de config, ou vérifiez le chemin.</p>
</div></aside>

### E1002 : Échec du parsing ferrflow.json

<span id="e1002"></span>

Le fichier `ferrflow.json` contient du JSON invalide.

### E1003 : Échec du parsing ferrflow.json5

<span id="e1003"></span>

Le fichier `ferrflow.json5` contient du JSON5 invalide.

### E1004 : Échec du parsing ferrflow.toml

<span id="e1004"></span>

Le fichier `ferrflow.toml` contient du TOML invalide.

### E1005 : Erreur de sérialisation TOML

<span id="e1005"></span>

Erreur interne lors de l'écriture TOML.

### E1006 : Échec du parsing .ferrflow

<span id="e1006"></span>

Le fichier `.ferrflow` contient du JSON invalide.

### E1007 : Erreur de sérialisation .ferrflow

<span id="e1007"></span>

Erreur interne lors de l'écriture du dotfile.

### E1008 : Résolution de chemin impossible

<span id="e1008"></span>

Un chemin dans la config n'a pas pu être résolu en chemin absolu.

### E1009 : Écriture du loader temporaire impossible

<span id="e1009"></span>

Impossible d'écrire le loader JS/TS temporaire.

### E1010 : Impossible d'exécuter tsx

<span id="e1010"></span>

Le runtime `tsx` est introuvable pour les configs `.ts`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Installez tsx : <code>npm install -g tsx</code>, ou utilisez un format JSON/TOML.</p>
</div></aside>

### E1011 : Impossible d'exécuter node

<span id="e1011"></span>

Le runtime `node` est introuvable pour les configs `.js`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Installez Node.js ou utilisez un format JSON/TOML.</p>
</div></aside>

### E1012 : Évaluation de la config échouée

<span id="e1012"></span>

Le fichier JS/TS a levé une erreur lors de l'évaluation.

### E1013 : Sortie de config invalide

<span id="e1013"></span>

Le fichier JS/TS a produit une sortie non UTF-8.

### E1014 : JSON invalide depuis la config

<span id="e1014"></span>

Le fichier JS/TS n'a pas produit de JSON valide.

### E1015 : Lecture du fichier impossible

<span id="e1015"></span>

Le fichier de config existe mais ne peut pas être lu.

### E1016 : Plusieurs fichiers de config

<span id="e1016"></span>

Plusieurs fichiers de config trouvés dans le répertoire.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Gardez un seul fichier de config.</p>
</div></aside>

### E1017 : Fichier déjà existant

<span id="e1017"></span>

`ferrflow init` lancé alors qu'un fichier de config existe déjà.

## Erreurs de validation

### E1100 : Spec de repo invalide

<span id="e1100"></span>

L'argument `--repo` ne correspond pas au format attendu `owner/repo`.

### E1101 : Erreur API GitHub

<span id="e1101"></span>

L'API GitHub a retourné une erreur lors de la validation distante.

### E1102 : Erreur API GitLab

<span id="e1102"></span>

L'API GitLab a retourné une erreur lors de la validation distante.

### E1103 : UTF-8 invalide

<span id="e1103"></span>

Le fichier de config distant contient un encodage UTF-8 invalide.

### E1104 : Parsing de la config distante échoué

<span id="e1104"></span>

Le fichier de config distant n'a pas pu être parsé.

### E1105 : Fichier de config distant introuvable

<span id="e1105"></span>

Le chemin spécifié n'existe pas dans le dépôt distant.

### E1106 : Aucun fichier de config trouvé

<span id="e1106"></span>

Aucun fichier de config FerrFlow dans le dépôt distant.

### E1107 : --ref nécessite --repo

<span id="e1107"></span>

Le flag `--ref` a été utilisé sans `--repo`.

## Opérations Git

### E2001 : Pas un dépôt git

<span id="e2001"></span>

Le répertoire courant n'est pas dans un dépôt git.

### E2002 : Dépôt bare non supporté

<span id="e2002"></span>

FerrFlow ne supporte pas les dépôts git bare.

### E2003 : Tag existant

<span id="e2003"></span>

Le tag que FerrFlow veut créer existe déjà.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Supprimez le tag existant ou utilisez <code>--force</code>.</p>
</div></aside>

### E2004 : Push de branche échoué

<span id="e2004"></span>

Impossible de push la branche de release.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez vos droits de push et les règles de protection.</p>
</div></aside>

### E2005 : Push rejeté

<span id="e2005"></span>

Le remote a rejeté le push.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Pullez les derniers changements et réessayez.</p>
</div></aside>

### E2006 : Push des tags échoué

<span id="e2006"></span>

Impossible de push les tags vers le remote.

### E2007 : Push des tags flottants échoué

<span id="e2007"></span>

Impossible de force-push les tags flottants.

### E2008 : Remote introuvable

<span id="e2008"></span>

Le remote git configuré n'existe pas.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez <code>git remote -v</code> et le champ <code>remote</code> de votre config.</p>
</div></aside>

### E2009 : Vérification post-push échouée

<span id="e2009"></span>

Le commit de release n'a pas pu être vérifié sur la branche distante.

### E2010 : Branche distante introuvable

<span id="e2010"></span>

La branche distante n'a pas été trouvée après le push.

## API GitHub

### E3001 : Création de release échouée

<span id="e3001"></span>

L'API GitHub Releases a retourné une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez que <code>GITHUB_TOKEN</code> a la permission <code>contents: write</code>.</p>
</div></aside>

### E3002 to E3010 : Erreurs API GitHub

<span id="e3002"></span>

Erreurs lors d'opérations sur l'API GitHub (releases, PR, auto-merge, GraphQL).

## API GitLab

### E3101 : Création de release échouée

<span id="e3101"></span>

L'API GitLab Releases a retourné une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez que le token CI a les accès API nécessaires.</p>
</div></aside>

### E3102 to E3105 : Erreurs API GitLab

<span id="e3102"></span>

Erreurs lors d'opérations sur l'API GitLab (releases, MR, merge).

## Fichiers de version

Les erreurs E4xxx concernent la lecture, l'écriture et le parsing des fichiers de version :

| Plage          | Format                            |
| -------------- | --------------------------------- |
| E4101 to E4105 | TOML (Cargo.toml, pyproject.toml) |
| E4201 to E4205 | JSON (package.json)               |
| E4301 to E4304 | Helm / YAML (Chart.yaml)          |
| E4401 to E4413 | XML / CSProj                      |
| E4501 to E4504 | Gradle                            |
| E4601 to E4603 | Go mod                            |
| E4701 to E4704 | Texte (VERSION, VERSION.txt)      |

Erreurs courantes : lecture impossible, syntaxe invalide, champ `version` manquant, écriture impossible, UTF-8 invalide.

## Pré-release

### E5001 : Nom de channel vide

<span id="e5001"></span>

Le nom du channel de pré-release est vide.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Spécifiez un nom : <code>--channel beta</code></p>
</div></aside>

### E5002 : Nom de channel invalide

<span id="e5002"></span>

Seuls les alphanumériques et tirets sont acceptés.

## Versioning

### E5010 : Semver invalide

<span id="e5010"></span>

La version actuelle n'est pas un semver valide.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Format attendu : <code>MAJEUR.MINEUR.PATCH</code>.</p>
</div></aside>

## Hooks

### E6001 : Hook échoué

<span id="e6001"></span>

Un hook a échoué avec `on_failure: "abort"`.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez la commande du hook, ou mettez <code>on_failure: &quot;continue&quot;</code>.</p>
</div></aside>

## Query

### E7001 : Aucun package configuré

<span id="e7001"></span>

Aucun package dans le fichier de config.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow init</code> ou ajoutez des packages manuellement.</p>
</div></aside>

### E7002 : Package introuvable

<span id="e7002"></span>

Le nom de package spécifié n'existe pas dans la config.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez <code>ferrflow version</code> pour lister les packages.</p>
</div></aside>

## Monorepo

### E8001 : Package introuvable dans la config

<span id="e8001"></span>

Un package référencé pendant la release n'a pas été trouvé.

### E8002 : Tag flottant régressif

<span id="e8002"></span>

Un tag flottant serait déplacé vers une version plus ancienne.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Utilisez <code>--force</code> pour ignorer la vérification.</p>
</div></aside>
