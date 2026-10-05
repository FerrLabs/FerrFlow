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

### E1018 : Chemin hors du dépôt

<span id="e1018"></span>

Un chemin de `versionedFiles` est absolu, ou sort de la racine du dépôt avec `..`. FerrFlow ne lit et n'écrit les fichiers de version qu'à l'intérieur du dépôt : écrivez le chemin relativement à la racine.

### E1019 : Le pattern d'include ne trouve aucun fichier

<span id="e1019"></span>

Une entrée de `include` ne correspond à aucun fichier. Les patterns sont résolus depuis le répertoire du fichier de config racine. Corrigez le chemin ou supprimez le pattern.

### E1020 : Fichier inclus invalide

<span id="e1020"></span>

Un fichier chargé via `include` ne décrit pas un package valide. Soit il déclare `workspace`, `include` ou `package`, qui n'ont leur place que dans la config racine, soit son contenu ne se désérialise pas en package. Un fichier inclus décrit un seul package, avec les mêmes clés qu'une entrée `package`.

### E1021 : Package inclus hors du dépôt

<span id="e1021"></span>

Le `path` d'un package inclus pointe hors de la racine du dépôt. Dans un fichier inclus, `path` est relatif au répertoire de ce fichier, et vaut ce répertoire quand il est vide.

### E1022 : Nom de package en double

<span id="e1022"></span>

Deux packages portent le même `name`, qu'ils viennent de la config racine ou d'un `include`. L'erreur indique les deux chemins. Renommez l'un des deux.

### E1023 : Package sans chemin

<span id="e1023"></span>

Un package de la config racine n'a pas de `path`. Renseignez-le relativement à la racine du dépôt, ou déplacez le package dans son propre fichier listé sous `include`, où `path` vaut par défaut le répertoire de ce fichier.

### E1024 : Fichier versionne introuvable

<span id="e1024"></span>

Un package que cette execution allait publier declare une entree `versionedFiles` dont le fichier n'est pas sur le disque. L'execution s'arrete au moment du plan plutot qu'au moment de l'ecriture, ou le meme probleme apparait sous la forme d'une simple erreur de lecture.

La cause habituelle est un chemin ecrit relativement au package plutot qu'a la racine du depot. `package.path` n'est pas un prefixe que FerrFlow ajoute pour vous :

```toml
[[package]]
name = "api"
path = "packages/api"

[[package.versioned_files]]
path = "Cargo.toml"              # faux, cherche a la racine du depot
# path = "packages/api/Cargo.toml"  # correct
```

L'erreur indique le chemin qu'elle suppose correct. `ferrflow validate` signale le meme probleme pour tous les packages configures, y compris ceux que cette execution n'aurait pas touches.

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

### E2011 : Release déjà en cours

<span id="e2011"></span>

Un autre `ferrflow release` détient le verrou de release `.git/ferrflow.lock`, ou le fichier de verrou n'a pas pu être créé. Un verrou laissé par un processus mort sur la même machine est repris automatiquement, et celui d'une autre machine au bout de six heures. Le message affiche le contenu du verrou pour savoir qui le détient.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Si aucune autre release ne tourne, supprimez le fichier de verrou ou relancez avec <code>--force-unlock</code>.</p>
</div></aside>

### E2012 : Force-push de la branche de release échoué

<span id="e2012"></span>

FerrFlow n'a pas pu force-push la branche de la PR ou MR de release. Le message reprend la sortie de git. Vérifiez que le token peut pousser sur cette branche et qu'aucune règle de protection n'y interdit le force-push.

### E2013 : Inspection de la branche de release impossible

<span id="e2013"></span>

Avant de réutiliser la branche de release, FerrFlow lance `git ls-remote`, `git fetch` et `git log` dessus pour vérifier qu'elle ne contient que ses propres commits. L'une de ces commandes a échoué. La release continue : FerrFlow affiche un avertissement et fait comme si la branche ne contenait que ses commits.

### E2014 : Suppression du tag distant échouée

<span id="e2014"></span>

`ferrflow rollback` n'a pas pu supprimer avec `git push --delete` un tag poussé par l'exécution échouée. Un tag absent du remote, ou qui pointe désormais vers un autre commit, est ignoré. Vérifiez que le token peut supprimer des tags, puis relancez le rollback.

### E2015 : Revert échoué

<span id="e2015"></span>

`ferrflow rollback` n'a pas pu faire `git revert` sur le commit de release, par exemple parce qu'il entre en conflit avec des changements plus récents. Terminez le revert à la main. FerrFlow ne pousse jamais le revert : poussez la branche vous-même après vérification.

### E2016 : Clone de shadow-release échoué

<span id="e2016"></span>

`ferrflow shadow-release` n'a pas pu cloner le dépôt dans un répertoire temporaire. Vérifiez que git est dans le `PATH` et que le répertoire temporaire du système est accessible en écriture.

## API GitHub

### E3001 : Création de release échouée

<span id="e3001"></span>

L'API GitHub Releases a retourné une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez que <code>GITHUB_TOKEN</code> a la permission <code>contents: write</code>.</p>
</div></aside>

### E3002 : Liste des releases échouée

<span id="e3002"></span>

Impossible de récupérer les releases existantes depuis l'API GitHub.

### E3003 : Réponse des releases illisible

<span id="e3003"></span>

L'API GitHub a renvoyé une réponse dans un format inattendu.

### E3004 : Publication de release échouée

<span id="e3004"></span>

Impossible de publier (sortir du brouillon) une release GitHub.

### E3005 : Création de pull request échouée

<span id="e3005"></span>

L'API GitHub a retourné une erreur à la création de la PR.

### E3006 : Réponse de PR illisible

<span id="e3006"></span>

L'API GitHub a renvoyé une PR dans un format inattendu.

### E3007 : Champ manquant dans la réponse de PR

<span id="e3007"></span>

La réponse de l'API GitHub pour la PR ne contient pas le champ `number` ou `node_id`.

### E3008 : Activation de l'auto-merge échouée

<span id="e3008"></span>

Impossible d'activer l'auto-merge sur la PR de release via l'API GraphQL.

### E3009 : Réponse GraphQL illisible

<span id="e3009"></span>

L'API GraphQL de GitHub a renvoyé une réponse inattendue.

### E3010 : Auto-merge échoué

<span id="e3010"></span>

La mutation GraphQL qui active l'auto-merge a renvoyé une erreur.

### E3011 : Recherche de la PR de release échouée

<span id="e3011"></span>

La liste des pull requests ouvertes n'a pas pu être récupérée pendant que FerrFlow cherchait une PR de release existante à mettre à jour. Vérifiez le token et l'accès à l'API GitHub.

### E3012 : Mise à jour de la PR de release échouée

<span id="e3012"></span>

FerrFlow a trouvé la PR de release ouverte, mais GitHub a refusé la mise à jour de son titre et de sa description. Vérifiez que le token a `pull-requests: write`.

### E3013 : Requête GraphQL échouée

<span id="e3013"></span>

Une requête vers l'API GraphQL de GitHub a échoué avant d'obtenir une réponse exploitable, sur le réseau ou avec un statut HTTP d'erreur. FerrFlow appelle cette API pour créer le commit de release avec `createCommitOnBranch` quand `FERRFLOW_BOT` est défini.

### E3014 : Erreur GraphQL

<span id="e3014"></span>

L'API GraphQL de GitHub a répondu par une erreur, ou `createCommitOnBranch` n'a renvoyé aucun identifiant de commit. Le message reprend le texte de l'erreur GitHub.

### E3015 : Déplacement de la branche de release échoué

<span id="e3015"></span>

Quand `FERRFLOW_BOT` est défini, FerrFlow fait pointer la branche de release sur la tête de la branche cible via l'API refs de GitHub avant d'y créer le commit. GitHub a refusé la mise à jour, ou la création de la ref quand la branche n'existait pas encore. Vérifiez que le token a `contents: write`.

### E3016 : Suppression de release échouée

<span id="e3016"></span>

`ferrflow rollback` n'a pas pu supprimer une release GitHub créée par l'exécution échouée. Supprimez-la à la main et relancez le rollback.

## API GitLab

### E3101 : Création de release échouée

<span id="e3101"></span>

L'API GitLab Releases a retourné une erreur.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Vérifiez que le token CI a les accès API nécessaires.</p>
</div></aside>

### E3102 : Création de merge request échouée

<span id="e3102"></span>

L'API GitLab a retourné une erreur à la création de la MR.

### E3103 : Réponse de MR illisible

<span id="e3103"></span>

L'API GitLab a renvoyé une MR dans un format inattendu.

### E3104 : Champ iid manquant

<span id="e3104"></span>

La réponse de l'API GitLab pour la MR ne contient pas le champ `iid`.

### E3105 : Merge de la MR échoué

<span id="e3105"></span>

Impossible de merger la MR de release via l'API GitLab.

### E3106 : Recherche de la MR de release échouée

<span id="e3106"></span>

La liste des merge requests ouvertes n'a pas pu être récupérée pendant que FerrFlow cherchait une MR de release existante à mettre à jour. Vérifiez le token et l'accès à l'API GitLab.

### E3107 : Mise à jour de la MR de release échouée

<span id="e3107"></span>

FerrFlow a trouvé la MR de release ouverte, mais GitLab a refusé la mise à jour de son titre et de sa description. Vérifiez les droits du token sur le projet.

## API Gitea

### E3201 : Création de release échouée

<span id="e3201"></span>

L'API Gitea Releases a retourné une erreur à la création d'une release. Vérifiez que le token peut écrire dans le dépôt.

### E3202 : Liste des releases échouée

<span id="e3202"></span>

Impossible de lister les releases depuis l'API Gitea pour chercher un brouillon de release existant pour le tag.

### E3203 : Publication de release échouée

<span id="e3203"></span>

Impossible de publier (sortir du brouillon) une release Gitea.

## API Bitbucket

### E3301 : Résolution du tag de release échouée

<span id="e3301"></span>

Bitbucket n'a pas de releases. Sur Bitbucket Cloud, FerrFlow interroge plutôt l'API sur le nouveau tag pour en obtenir le lien, et cette requête a échoué. Vérifiez que le token peut lire le dépôt et que le tag a bien été poussé.

## Fichiers de version

Les erreurs E4xxx concernent la lecture, l'écriture et le parsing des fichiers de version. Chaque format a sa propre plage de codes. Les codes UTF-8 invalide viennent de `ferrflow validate`, qui lit les fichiers versionnés comme des octets bruts ; les autres commandes signalent un fichier qui n'est pas en UTF-8 avec l'erreur de lecture du format.

### E4101 : Lecture du fichier TOML impossible

<span id="e4101"></span>

FerrFlow n'a pas pu lire le fichier de version TOML (`Cargo.toml`, `pyproject.toml`) sur le disque.

### E4102 : Syntaxe TOML invalide

<span id="e4102"></span>

Le fichier de version TOML ne se parse pas. Le message indique où se trouve l'erreur.

### E4103 : Aucune version dans le fichier TOML

<span id="e4103"></span>

Aucun des champs `package.version`, `workspace.package.version`, `project.version` ou `tool.poetry.version` n'est renseigné. Un `Cargo.toml` qui déclare `version.workspace = true` tombe aussi ici quand le même fichier n'a pas de `[workspace.package].version` : faites pointer le fichier versionné vers le `Cargo.toml` racine du workspace.

### E4104 : Écriture du fichier TOML impossible

<span id="e4104"></span>

L'écriture de la nouvelle version dans le fichier de version TOML (`Cargo.toml`, `pyproject.toml`) a échoué.

### E4105 : UTF-8 invalide dans le fichier TOML

<span id="e4105"></span>

Le contenu de le fichier de version TOML (`Cargo.toml`, `pyproject.toml`) n'est pas de l'UTF-8 valide.

### E4201 : Lecture du fichier JSON impossible

<span id="e4201"></span>

FerrFlow n'a pas pu lire le fichier de version JSON (`package.json`, `composer.json`) sur le disque. Le même code couvre le checkpoint de release `.git/ferrflow.checkpoint.json`.

### E4202 : Syntaxe JSON invalide

<span id="e4202"></span>

Le fichier de version JSON ne se parse pas. FerrFlow lève aussi ce code quand le checkpoint de release `.git/ferrflow.checkpoint.json` est corrompu ou a été écrit par une autre version de FerrFlow : supprimez ce fichier pour repartir de zéro.

### E4203 : Aucune version dans le fichier JSON

<span id="e4203"></span>

Le fichier JSON n'a pas de champ `version` au premier niveau.

### E4204 : Écriture du fichier JSON impossible

<span id="e4204"></span>

L'écriture de la nouvelle version dans le fichier de version JSON (`package.json`, `composer.json`) a échoué. FerrFlow le lève aussi quand il ne peut pas enregistrer ou supprimer le checkpoint de release, ou réécrire le manifeste d'un package dépendant (`package.json` ou `Cargo.toml`) pour suivre une dépendance bumpée.

### E4205 : UTF-8 invalide dans le fichier JSON

<span id="e4205"></span>

Le contenu de le fichier de version JSON (`package.json`, `composer.json`) n'est pas de l'UTF-8 valide.

### E4301 : Lecture de Chart.yaml (helm) impossible

<span id="e4301"></span>

FerrFlow n'a pas pu lire le `Chart.yaml` déclaré avec `format = "helm"` sur le disque.

### E4302 : Aucune version dans Chart.yaml (helm)

<span id="e4302"></span>

Le `Chart.yaml` déclaré avec `format = "helm"` n'a pas de champ `version:` au premier niveau.

### E4303 : Écriture de Chart.yaml (helm) impossible

<span id="e4303"></span>

L'écriture de la nouvelle version dans le `Chart.yaml` déclaré avec `format = "helm"` a échoué.

### E4304 : UTF-8 invalide dans Chart.yaml (helm)

<span id="e4304"></span>

Le contenu de le `Chart.yaml` déclaré avec `format = "helm"` n'est pas de l'UTF-8 valide.

### E4401 : Lecture du fichier XML impossible

<span id="e4401"></span>

FerrFlow n'a pas pu lire le fichier de version XML (`pom.xml`) sur le disque.

### E4402 : Aucune version dans le fichier XML

<span id="e4402"></span>

Le fichier XML n'a pas de balise `<version>`.

### E4403 : Écriture du fichier XML impossible

<span id="e4403"></span>

L'écriture de la nouvelle version dans le fichier de version XML (`pom.xml`) a échoué.

### E4404 : UTF-8 invalide dans le fichier XML

<span id="e4404"></span>

Le contenu de le fichier de version XML (`pom.xml`) n'est pas de l'UTF-8 valide.

### E4410 : Lecture du fichier .csproj impossible

<span id="e4410"></span>

FerrFlow n'a pas pu lire le fichier `.csproj` sur le disque.

### E4411 : Aucune version dans le fichier .csproj

<span id="e4411"></span>

Le fichier `.csproj` n'a pas d'élément `<Version>`.

### E4412 : Écriture du fichier .csproj impossible

<span id="e4412"></span>

L'écriture de la nouvelle version dans le fichier `.csproj` a échoué.

### E4413 : UTF-8 invalide dans le fichier .csproj

<span id="e4413"></span>

Le contenu de le fichier `.csproj` n'est pas de l'UTF-8 valide.

### E4501 : Lecture du fichier Gradle impossible

<span id="e4501"></span>

FerrFlow n'a pas pu lire le fichier de build Gradle (`build.gradle`, `build.gradle.kts`) sur le disque.

### E4502 : Aucune version dans le fichier Gradle

<span id="e4502"></span>

Le fichier de build Gradle n'a pas d'affectation `version = "…"`.

### E4503 : Écriture du fichier Gradle impossible

<span id="e4503"></span>

L'écriture de la nouvelle version dans le fichier de build Gradle (`build.gradle`, `build.gradle.kts`) a échoué.

### E4504 : UTF-8 invalide dans le fichier Gradle

<span id="e4504"></span>

Le contenu de le fichier de build Gradle (`build.gradle`, `build.gradle.kts`) n'est pas de l'UTF-8 valide.

### E4601 : Impossible de lancer git describe

<span id="e4601"></span>

Un fichier versionné `gomod` tire sa version des tags git via `git describe`, et la commande n'a pas pu être lancée. Vérifiez que git est dans le `PATH`.

### E4602 : Aucun tag de version

<span id="e4602"></span>

`git describe` n'a trouvé aucun tag correspondant à `*@v*` ou `v*` pour le package.

Depuis FerrFlow v3, le flux de release intercepte ce cas et repart de la version de départ de la stratégie (`0.0.0`, `0`, …) : la première release d'un dépôt neuf réussit sans tag créé à la main. Le code existe toujours pour les programmes qui utilisent `GoModVersionFile` directement, mais `ferrflow release` ne devrait plus l'afficher.

### E4603 : Version illisible depuis go.mod

<span id="e4603"></span>

La version d'un module Go vient des tags git : FerrFlow ne peut pas la lire dans le contenu de `go.mod`. `ferrflow validate` ignore les entrées `gomod` avec un avertissement.

### E4701 : Lecture du fichier texte impossible

<span id="e4701"></span>

FerrFlow n'a pas pu lire le fichier de version texte (`VERSION`, `VERSION.txt`) sur le disque.

### E4702 : Aucune version dans le fichier texte

<span id="e4702"></span>

Le fichier texte est vide. Avec un `selector`, ce code couvre aussi une regex qui ne compile pas, qui n'a pas exactement un groupe de capture, qui ne trouve rien ou qui ne capture que des espaces.

### E4703 : Écriture du fichier texte impossible

<span id="e4703"></span>

L'écriture de la nouvelle version dans le fichier de version texte (`VERSION`, `VERSION.txt`) a échoué.

### E4704 : UTF-8 invalide dans le fichier texte

<span id="e4704"></span>

Le contenu de le fichier de version texte (`VERSION`, `VERSION.txt`) n'est pas de l'UTF-8 valide.

### E4801 : Lecture de pubspec.yaml impossible

<span id="e4801"></span>

FerrFlow n'a pas pu lire `pubspec.yaml` sur le disque.

### E4802 : Aucune version dans pubspec.yaml

<span id="e4802"></span>

`pubspec.yaml` n'a pas de clé `version:` au premier niveau.

### E4803 : Écriture de pubspec.yaml impossible

<span id="e4803"></span>

L'écriture de la nouvelle version dans `pubspec.yaml` a échoué.

### E4804 : UTF-8 invalide dans pubspec.yaml

<span id="e4804"></span>

Le contenu de `pubspec.yaml` n'est pas de l'UTF-8 valide.

### E4811 : Lecture de mix.exs impossible

<span id="e4811"></span>

FerrFlow n'a pas pu lire `mix.exs` sur le disque.

### E4812 : Aucune version dans mix.exs

<span id="e4812"></span>

`mix.exs` ne contient pas de littéral `version: "…"`.

### E4813 : Écriture de mix.exs impossible

<span id="e4813"></span>

L'écriture de la nouvelle version dans `mix.exs` a échoué.

### E4814 : UTF-8 invalide dans mix.exs

<span id="e4814"></span>

Le contenu de `mix.exs` n'est pas de l'UTF-8 valide.

### E4821 : Lecture de Chart.yaml (chartyaml) impossible

<span id="e4821"></span>

FerrFlow n'a pas pu lire le `Chart.yaml` déclaré avec `format = "chartyaml"` sur le disque.

### E4822 : Aucune version dans Chart.yaml (chartyaml)

<span id="e4822"></span>

Le `Chart.yaml` déclaré avec `format = "chartyaml"` n'a pas de clé `version:` au premier niveau.

### E4823 : Écriture de Chart.yaml (chartyaml) impossible

<span id="e4823"></span>

L'écriture de la nouvelle version dans le `Chart.yaml` déclaré avec `format = "chartyaml"` a échoué.

### E4824 : UTF-8 invalide dans Chart.yaml (chartyaml)

<span id="e4824"></span>

Le contenu de le `Chart.yaml` déclaré avec `format = "chartyaml"` n'est pas de l'UTF-8 valide.

### E4831 : Lecture du fichier .gemspec impossible

<span id="e4831"></span>

FerrFlow n'a pas pu lire le fichier `.gemspec` sur le disque.

### E4832 : Aucune version dans le fichier .gemspec

<span id="e4832"></span>

Le fichier `.gemspec` n'a pas d'affectation `<ident>.version = "…"`.

### E4833 : Écriture du fichier .gemspec impossible

<span id="e4833"></span>

L'écriture de la nouvelle version dans le fichier `.gemspec` a échoué.

### E4834 : UTF-8 invalide dans le fichier .gemspec

<span id="e4834"></span>

Le contenu de le fichier `.gemspec` n'est pas de l'UTF-8 valide.

### E4841 : Lecture de Package.swift impossible

<span id="e4841"></span>

FerrFlow n'a pas pu lire `Package.swift` sur le disque.

### E4842 : Aucune version dans Package.swift

<span id="e4842"></span>

`Package.swift` n'a pas de déclaration `let <name>Version = "…"` au premier niveau.

### E4843 : Écriture de Package.swift impossible

<span id="e4843"></span>

L'écriture de la nouvelle version dans `Package.swift` a échoué.

### E4844 : UTF-8 invalide dans Package.swift

<span id="e4844"></span>

Le contenu de `Package.swift` n'est pas de l'UTF-8 valide.

### E4851 : Lecture du fichier .cabal impossible

<span id="e4851"></span>

FerrFlow n'a pas pu lire le fichier `.cabal` sur le disque.

### E4852 : Aucune version dans le fichier .cabal

<span id="e4852"></span>

Le fichier `.cabal` n'a pas de champ `version:` au premier niveau.

### E4853 : Écriture du fichier .cabal impossible

<span id="e4853"></span>

L'écriture de la nouvelle version dans le fichier `.cabal` a échoué.

### E4854 : UTF-8 invalide dans le fichier .cabal

<span id="e4854"></span>

Le contenu de le fichier `.cabal` n'est pas de l'UTF-8 valide.

### E4861 : Lecture de CMakeLists.txt impossible

<span id="e4861"></span>

FerrFlow n'a pas pu lire `CMakeLists.txt` sur le disque.

### E4862 : Aucune version dans CMakeLists.txt

<span id="e4862"></span>

`CMakeLists.txt` n'a pas de déclaration `project(… VERSION …)`.

### E4863 : Écriture de CMakeLists.txt impossible

<span id="e4863"></span>

L'écriture de la nouvelle version dans `CMakeLists.txt` a échoué.

### E4864 : UTF-8 invalide dans CMakeLists.txt

<span id="e4864"></span>

Le contenu de `CMakeLists.txt` n'est pas de l'UTF-8 valide.

### E4871 : Lecture de galaxy.yml impossible

<span id="e4871"></span>

FerrFlow n'a pas pu lire `galaxy.yml` sur le disque.

### E4872 : Aucune version dans galaxy.yml

<span id="e4872"></span>

`galaxy.yml` n'a pas de clé `version:` au premier niveau.

### E4873 : Écriture de galaxy.yml impossible

<span id="e4873"></span>

L'écriture de la nouvelle version dans `galaxy.yml` a échoué.

### E4874 : UTF-8 invalide dans galaxy.yml

<span id="e4874"></span>

Le contenu de `galaxy.yml` n'est pas de l'UTF-8 valide.

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

## Publishers

### E6101 : Publisher mal configuré

<span id="e6101"></span>

Un publisher n'a pas pu démarrer parce que sa configuration est incomplète : un `registry` absent de `workspace.registries`, une variable `tokenEnv` non définie, un contexte de build, un chart ou un fichier d'asset introuvable, ou un `trustedPublishing` utilisé hors de GitHub Actions. Le message nomme le publisher et l'élément manquant. Rien n'a été publié.

### E6102 : Publication échouée

<span id="e6102"></span>

L'étape de publication a tourné et a échoué : `cargo publish`, `npm publish`, `twine upload`, `docker buildx`, `helm push`, `gh release upload` ou un webhook a renvoyé une erreur. Le message reprend l'erreur de l'outil, avec les lignes qui en expliquent la cause. Pour cargo, FerrFlow réessaie quelques fois quand l'erreur ressemble à un retard d'index du registre juste après la publication d'une dépendance, et signale E6102 une fois les essais épuisés.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Lancez la même commande à la main depuis le dossier du package, par exemple <code>cargo publish --dry-run</code>, pour voir la sortie complète.</p>
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

### E7003 : Plage de diff invalide

<span id="e7003"></span>

`ferrflow diff` attend une plage de la forme `<from>..<to>`, avec deux bornes non vides, par exemple `v1.4.0..v1.6.0`.

### E7004 : Nom de package requis

<span id="e7004"></span>

Le dépôt est un monorepo : `ferrflow diff` doit savoir quel package comparer, avec `ferrflow diff <package> <from>..<to>`.

### E7005 : Borne de plage introuvable

<span id="e7005"></span>

Une borne de la plage de `ferrflow diff` ne correspond à aucun tag. FerrFlow essaie la valeur telle quelle, puis comme une version passée dans le modèle de tag du package, avec et sans `v` initial. Le message liste les noms essayés. Passez un nom de tag ou une version existants.

## Monorepo

### E8001 : Package introuvable dans la config

<span id="e8001"></span>

Un package référencé pendant la release n'a pas été trouvé.

### E8002 : Tag flottant régressif

<span id="e8002"></span>

Un tag flottant serait déplacé vers une version plus ancienne.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Utilisez <code>--force</code> pour ignorer la vérification.</p>
</div></aside>

### E8003 : Cycle de dépendances

<span id="e8003"></span>

Deux packages ou plus dépendent l'un de l'autre via `dependsOn`, directement ou transitivement : il n'existe aucun ordre de release possible. Le message nomme la boucle, par exemple `cycle detected: api → web → api`. Supprimez l'une des arêtes `dependsOn` pour la casser. La vérification s'exécute avant toute écriture de version, donc une configuration cyclique ne laisse jamais de release partielle.

## Rollback

### E10000 : Rollback bloqué

<span id="e10000"></span>

Chaque package de l'exécution échouée a publié sur un registre qui ne permet pas de dépublier : `ferrflow rollback` n'a rien qu'il puisse annuler. Le plan affiché indique chaque package bloqué et la raison.
