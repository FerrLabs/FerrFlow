---
title: Telemetrie
description: Ce que FerrFlow collecte, comment les donnees sont anonymisees, et comment desactiver la telemetrie.
---

FerrFlow collecte des donnees de telemetrie anonymes pour ameliorer l'outil. Cette page explique exactement ce qui est envoye, comment les donnees sont anonymisees, et comment desactiver la telemetrie.

## Ce qui est collecte

A chaque execution d'une commande, FerrFlow peut envoyer un evenement contenant :

| Champ           | Description                                                       |
| --------------- | ----------------------------------------------------------------- |
| `event_type`    | L'action effectuee : `check`, `release`, `version_bump` ou `init` |
| `commits_count` | Nombre de commits depuis la derniere release                      |
| `repo_hash`     | Un hash SHA-256 de l'URL du remote git (voir ci-dessous)          |

Seuls les champs pertinents sont inclus. Les champs vides sont omis.

## Comment les donnees sont anonymisees

L'URL de votre depot n'est **jamais envoyee en clair**. FerrFlow calcule un hash SHA-256 de l'URL du remote git et envoie uniquement le digest hexadecimal. Cela permet de compter les depots uniques sans savoir lesquels ils sont.

Aucun code source, nom de fichier, message de commit, nom de branche, nom de package, numero de version, adresse IP ou information personnelle n'est collecte ou stocke.

## Ou les donnees sont envoyees

Les evenements sont envoyes via une requete POST a `https://api.ferrflow.com/events`. La requete est asynchrone et non-bloquante — elle ne ralentit jamais votre workflow. Si la requete echoue, elle est silencieusement ignoree.

## Comment desactiver

Vous pouvez desactiver completement la telemetrie via une variable d'environnement ou votre fichier de configuration.

### Variable d'environnement

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="Linux / macOS"><p class="ferr-tab__label">Linux / macOS</p><div class="ferr-tab__body"><pre><code class="language-bash">export FERRFLOW_ANONYMOUS_TELEMETRY=false
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="Windows"><p class="ferr-tab__label">Windows</p><div class="ferr-tab__body"><pre><code class="language-powershell">$env:FERRFLOW_ANONYMOUS_TELEMETRY = &quot;false&quot;
</code></pre>
</div></div>
</div>

Valeurs acceptees pour desactiver : `false`, `0`, `off`, `no` (insensible a la casse).

Depuis la v4.10, FerrFlow honore aussi la variable d'environnement standard [`DO_NOT_TRACK`](https://consoledonottrack.com/) — `DO_NOT_TRACK=1` desactive la telemetrie sans aucune configuration specifique. `FERRFLOW_TELEMETRY=false` reste pris en charge comme alternative pour la retrocompatibilite avec les configs v0/v1.

### Fichier de configuration

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;workspace&quot;: {
    &quot;anonymous_telemetry&quot;: false
  }
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[workspace]
anonymous_telemetry = false
</code></pre>
</div></div>
</div>

L'une ou l'autre methode suffit pour desactiver la telemetrie.
