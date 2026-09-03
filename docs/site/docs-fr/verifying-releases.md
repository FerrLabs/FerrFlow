---
title: Vérifier les releases
description: Chaque release FerrFlow embarque des signatures Sigstore, un SBOM CycloneDX et une attestation de provenance SLSA. Voici comment les vérifier.
---

Chaque tarball de release, l'image Docker, l'archive de complétions binaires et le SBOM qui les accompagne sont signés via [Sigstore](https://www.sigstore.dev/) en signature sans clé. Il n'y a aucune clé publique à suivre : l'identité de signature est celle du workload GitHub Actions qui tourne dans `FerrLabs/FerrFlow`, ancrée dans le journal public de transparence [Rekor](https://docs.sigstore.dev/logging/overview/).

Disponible depuis la v5.2.

## Ce qui est livré par release

| Artefact                           | Sidecars                           |
| ---------------------------------- | ---------------------------------- |
| `ferrflow-linux-x64.tar.gz`        | `.sig`, `.crt`                     |
| `ferrflow-linux-arm64.tar.gz`      | `.sig`, `.crt`                     |
| `ferrflow-linux-armv7.tar.gz`      | `.sig`, `.crt`                     |
| `ferrflow-darwin-x64.tar.gz`       | `.sig`, `.crt`                     |
| `ferrflow-darwin-arm64.tar.gz`     | `.sig`, `.crt`                     |
| `ferrflow-windows-x64.zip`         | `.sig`, `.crt`                     |
| `ferrflow-windows-arm64.zip`       | `.sig`, `.crt`                     |
| `ferrflow-completions.tar.gz`      | `.sig`, `.crt`                     |
| `sbom.cdx.json`                    | `.sig`, `.crt`                     |
| `ghcr.io/ferrlabs/ferrflow:vX.Y.Z` | Signature cosign dans GHCR + Rekor |

Tous les sidecars sont téléchargeables depuis la page GitHub Release à côté du binaire.

> Les releases jusqu'à **v5.47.4** embarquent une paire `.sig` + `.crt` au lieu d'un unique `.sigstore.json`. Vérifiez-les avec `--certificate <fichier>.crt --signature <fichier>.sig` à la place de `--bundle`. Le changement vient de cosign v3, qui a remplacé les sorties signature et certificat séparées par un bundle unique.

## Vérifier un tarball

```bash
# installer cosign (une seule fois)
curl -L https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64 \
  -o /usr/local/bin/cosign && chmod +x /usr/local/bin/cosign

# télécharger l'artefact + les sidecars depuis la page de release
TAG=v5.2.3
gh release download "$TAG" --repo FerrLabs/FerrFlow \
  -p 'ferrflow-linux-x64.tar.gz*'

# vérifier
cosign verify-blob \
  --bundle ferrflow-linux-x64.tar.gz.sigstore.json \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ferrflow-linux-x64.tar.gz
# → Verified OK
```

Une vérification qui passe signifie :

- Les octets du tarball n'ont pas été modifiés depuis que le workflow de release les a signés.
- L'identité de signature était un workflow exécuté dans `FerrLabs/FerrFlow` déclenché par l'issuer OIDC de GitHub Actions.
- La signature est inscrite dans le journal public Rekor : cherchez la valeur du `.sig` sur [search.sigstore.dev](https://search.sigstore.dev/).

## Vérifier l'image Docker

```bash
cosign verify ghcr.io/ferrlabs/ferrflow:v5.2.3 \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

## Vérifier le SBOM

Le SBOM (`sbom.cdx.json`) est un document [CycloneDX](https://cyclonedx.org/) listant toutes les dépendances transitives du binaire publié. Il est signé de la même façon que les tarballs :

```bash
cosign verify-blob \
  --bundle sbom.cdx.json.sigstore.json \
  --certificate-identity-regexp "https://github.com/FerrLabs/FerrFlow/.*" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  sbom.cdx.json
```

Injectez le SBOM vérifié dans votre scanner : Grype, Trivy, Snyk, JFrog Xray, Anchore, n'importe quoi qui parle CycloneDX.

## Attestation de provenance SLSA

En plus des signatures Sigstore, chaque release embarque aussi une [attestation de provenance SLSA](https://slsa.dev/) générée via [`actions/attest-build-provenance`](https://github.com/actions/attest-build-provenance). Elle enregistre le run du workflow, le SHA du commit source et les inputs de build.

```bash
gh attestation verify ferrflow-linux-x64.tar.gz --repo FerrLabs/FerrFlow
```

## Ce qui n'est pas signé

Les tarballs sources de `git archive` (les actifs « Source code (zip) » / « Source code (tar.gz) » générés automatiquement sur la page GitHub Release) viennent de GitHub, pas du workflow de release, et n'ont pas de sidecar de signature. Si vous avez besoin d'une attestation sur la source, utilisez plutôt `gh attestation verify` contre le bundle de provenance.

## Pourquoi c'est important

- **Attaques de chaîne d'approvisionnement.** Un attaquant qui compromet un CDN, un miroir ou pousse un package typosquat ne peut pas forger la signature : l'identité de signature est ancrée au flow OIDC de GitHub Actions.
- **Conformité.** Les clients SOC 2 / ISO 27001 peuvent attester que le binaire qu'ils ont récupéré est celui que leur auditeur a approuvé.
- **Aucune gestion de clé.** Personne chez FerrLabs n'a de clé privée de signature à perdre ou faire tourner. Le workflow prouve son identité au moment de la signature.

<aside class="ferr-aside ferr-aside--tip"><div class="ferr-aside__body"><p>Si vous déployez FerrFlow dans un environnement régulé, pinnez à la fois la version <strong>et</strong> la paire <code>.sig</code>/<code>.crt</code> dans votre étape de provisioning. Une compromission future du workflow ne peut pas antidater une signature déjà présente dans Rekor.</p>
</div></aside>
