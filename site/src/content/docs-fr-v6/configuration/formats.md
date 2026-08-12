---
title: Formats supportés
description: Formats de fichiers de version que FerrFlow peut lire et mettre à jour.
---

## TOML

Utilisé par Rust (`Cargo.toml`) et Python (`pyproject.toml`).

FerrFlow met à jour le champ `version` sous `[package]`, `[project]` ou `[tool.poetry]`.

```toml
[package]
name = "my-crate"
version = "1.2.3"   # ← mis à jour
```

## JSON

Utilisé par Node.js (`package.json`).

FerrFlow met à jour le champ `version` de premier niveau.

```json
{
  "name": "my-package",
  "version": "1.2.3"
}
```

## XML

Utilisé par Java/Maven (`pom.xml`).

FerrFlow met à jour le premier élément `<version>` rencontré.

```xml
<project>
  <groupId>com.example</groupId>
  <artifactId>my-app</artifactId>
  <version>1.2.3</version>   <!-- mis à jour -->
</project>
```

## Gradle

Utilisé par les projets Java/Kotlin Gradle (`build.gradle`, `build.gradle.kts`).

FerrFlow met à jour l'assignation `version = "..."`.

```groovy
version = "1.2.3"   // mis à jour
```

## Texte brut

Utilisé pour les fichiers de version simples (`VERSION`, `VERSION.txt`).

FerrFlow remplace l'intégralité du contenu du fichier par le numéro de version.

```
1.2.3
```

## Go modules

Utilisé par les projets Go (`go.mod`).

Les modules Go utilisent directement les tags git — FerrFlow ne modifie **pas** `go.mod`. La version est dérivée entièrement du tag git (`v1.2.3` ou `{name}@v1.2.3`).

## Helm

Utilisé par les charts Helm Kubernetes (`Chart.yaml`).

FerrFlow met à jour le champ `version` et, lorsqu'il est présent, maintient `appVersion` synchronisé.

```yaml
apiVersion: v2
name: my-app
version: 1.2.3 # ← mis à jour
appVersion: '1.2.3' # ← mis à jour si présent
```

## Cabal

Utilisé par les packages Haskell (`*.cabal`).

FerrFlow met à jour le champ `version` de premier niveau. Le champ `cabal-version`, qui déclare le format du fichier et non la version du package, n'est jamais modifié — pas plus qu'un `version:` indenté à l'intérieur d'une stanza.

```
cabal-version:      2.4   # ← laissé tel quel
name:               my-package
version:            1.2.3 # ← mis à jour
```

## CMake

Utilisé par les projets C / C++ (`CMakeLists.txt`).

FerrFlow met à jour l'argument `VERSION` de l'appel `project()`, y compris lorsqu'il est réparti sur plusieurs lignes. `cmake_minimum_required(VERSION …)` — la version minimale de l'outil CMake — n'est pas touché.

```cmake
cmake_minimum_required(VERSION 3.20)   # ← laissé tel quel

project(MyProject
    VERSION 1.2.3                      # ← mis à jour
    LANGUAGES CXX)
```

## Plusieurs fichiers par package

Un package peut avoir autant d'entrées de fichiers versionnés que nécessaire :

<div class="ferr-tabs">
  <div class="ferr-tab" data-label="JSON"><p class="ferr-tab__label">JSON</p><div class="ferr-tab__body"><pre><code class="language-json">{
  &quot;package&quot;: [
    {
      &quot;versionedFiles&quot;: [
        { &quot;path&quot;: &quot;Cargo.toml&quot;, &quot;format&quot;: &quot;toml&quot; },
        { &quot;path&quot;: &quot;npm/package.json&quot;, &quot;format&quot;: &quot;json&quot; }
      ]
    }
  ]
}
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="TOML"><p class="ferr-tab__label">TOML</p><div class="ferr-tab__body"><pre><code class="language-toml">[[package.versioned_files]]
path   = &quot;Cargo.toml&quot;
format = &quot;toml&quot;

[[package.versioned_files]]
path = &quot;npm/package.json&quot;
format = &quot;json&quot;
</code></pre>
</div></div>
  <div class="ferr-tab" data-label="JSON5"><p class="ferr-tab__label">JSON5</p><div class="ferr-tab__body"><pre><code class="language-json5">{
  package: [
    {
      versionedFiles: [
        { path: &quot;Cargo.toml&quot;, format: &quot;toml&quot; },
        { path: &quot;npm/package.json&quot;, format: &quot;json&quot; },
      ],
    },
  ],
}
</code></pre>
</div></div>
</div>

Les deux fichiers seront mis à jour avec la même version avant le commit git.
