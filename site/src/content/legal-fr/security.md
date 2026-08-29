---
title: 'Sécurité : FerrFlow'
description: Comment signaler une vulnérabilité dans FerrFlow, le modèle de menace de la CLI, et la posture sécurité de FerrFlow en tant qu'outil open source.
lastUpdated: 2026-05-14
---

<p>
  FerrFlow est une CLI open source. Elle s'exécute entièrement sur votre machine ou dans votre
  runner CI : votre code source, vos commits, vos tags. Rien ne quitte votre infrastructure,
  rien ne téléphone à la maison. Cette page indique comment la chaîne d'approvisionnement du
  binaire est protégée, où signaler une vulnérabilité, et ce qui est dans le périmètre.
</p>

<h2>Posture</h2>
<p>
  FerrFlow n'a aucun serveur à protéger. Pas de compte FerrFlow, pas de base de données, pas
  d'endpoint API manipulant des données utilisateur. La CLI lit votre historique git, modifie
  les chaînes de version dans des fichiers que vous contrôlez, pousse des tags via vos
  identifiants git existants, puis sort. Le site compagnon <strong>ferrflow.com</strong> est
  une surface de documentation statique : voir la
  <a href="/fr/privacy">politique de confidentialité</a> pour le peu qu'il journalise.
</p>
<p>
  Pour la posture sécurité plus large de la plateforme FerrLabs (chiffrement, identité,
  isolation réseau, journalisation d'audit, gestion des sous-traitants), voir
  <a href="https://ferrlabs.com/fr/security">ferrlabs.com/fr/security</a>. FerrFlow hérite très
  peu de cette posture en pratique (c'est une CLI, pas un SaaS) mais la page canonique reste
  la source de vérité pour les contrôles FerrLabs.
</p>

<h2>Modèle de menace</h2>
<p>
  La principale surface de risque est la <strong>chaîne d'approvisionnement du binaire</strong>.
  Un build FerrFlow malveillant dans un runner CI pourrait lire le contenu du dépôt, pousser
  des tags ou publier des releases avec les identifiants du projet. Mesures d'atténuation :
</p>
<ul>
  <li>
    Chaque release est construite dans <strong>GitHub Actions</strong> à partir du dépôt public
    <a href="https://github.com/FerrLabs/FerrFlow">FerrLabs/FerrFlow</a>, avec attestation de
    provenance (cible SLSA L3).
  </li>
  <li>
    Les binaires et images de conteneurs sont <strong>signés</strong> ; les checksums et
    signatures sont publiés avec chaque release sur
    <a href="https://github.com/FerrLabs/FerrFlow/releases">github.com/FerrLabs/FerrFlow/releases</a>.
  </li>
  <li>
    Le bundle self-host <code>ghcr.io/ferrlabs/ferrflow-selfhost</code> est distribué
    uniquement via GHCR, avec des signatures d'image vérifiables via <code>cosign</code>.
  </li>
  <li>
    Le JSON Schema servi à <code>ferrflow.com/schema/ferrflow.json</code> est diffusé depuis la
    même origine statique que ce site et est strictement identique octet pour octet à
    <code>schema/ferrflow.json</code> dans le dépôt source.
  </li>
</ul>

<h2>Signaler une vulnérabilité</h2>
<p>Deux canaux : choisissez celui qui correspond à votre workflow. Les deux sont surveillés.</p>
<ul>
  <li>
    <strong>E-mail</strong> : <a href="mailto:security@ferrlabs.com">security&#64;ferrlabs.com</a>.
    Empreinte de clé PGP et politique sur
    <a href="https://ferrlabs.com/.well-known/security.txt">ferrlabs.com/.well-known/security.txt</a>.
  </li>
  <li>
    <strong>Security advisory privé GitHub</strong> :
    <a href="https://github.com/FerrLabs/FerrFlow/security/advisories/new">déposez un advisory privé</a>
    directement sur le dépôt. Canal préféré si vous reproduisez déjà le problème sur une release
    taguée.
  </li>
</ul>
<p>
  Nous appliquons une <strong>divulgation coordonnée avec un délai par défaut de 90 jours</strong>.
  Accusé de réception sous 3 jours ouvrés, calendrier de divulgation négocié, crédit donné au
  rapporteur dans l'advisory publié sauf demande d'anonymat. Merci de ne pas ouvrir d'issue
  publique GitHub pour les vulnérabilités.
</p>

<h2>Périmètre</h2>
<p><strong>Dans le périmètre :</strong></p>
<ul>
  <li>
    Le code source de la CLI <a href="https://github.com/FerrLabs/FerrFlow">FerrLabs/FerrFlow</a>
    et les binaires qu'il produit.
  </li>
  <li>Les images de conteneurs publiées sur <code>ghcr.io/ferrlabs/ferrflow*</code>.</li>
  <li>Le JSON Schema servi à <code>ferrflow.com/schema/ferrflow.json</code>.</li>
  <li>Ce site marketing (<code>ferrflow.com</code>).</li>
</ul>
<p><strong>Hors périmètre :</strong></p>
<ul>
  <li>
    Les formats de fichiers tiers que FerrFlow lit ou met à jour (<code>Cargo.toml</code>,
    <code>package.json</code>, <code>pyproject.toml</code>, <code>Chart.yaml</code>,
    <code>mix.exs</code>, <code>pubspec.yaml</code>, gemspecs, etc.). Les vulnérabilités de ces
    écosystèmes relèvent de l'amont.
  </li>
  <li>
    Bugs de parsing Conventional Commits qui ne sont pas des problèmes de sécurité (ce sont des
    bug reports normaux).
  </li>
  <li>Déni de service en passant un input pathologique à la CLI sur votre propre machine.</li>
</ul>

<h2>Aucune donnée utilisateur</h2>
<p>
  <strong>FerrFlow ne collecte, ne transmet et ne stocke aucune donnée utilisateur.</strong> Pas
  de télémétrie, pas de reporting d'erreur, pas d'analytics d'usage, pas de phone-home. La seule
  activité réseau de la CLI consiste en les opérations git que vous invoquez explicitement
  (fetch, push de tags, création de releases GitHub via votre propre token) et en appels HTTP
  vers les registries de paquets que <em>vous</em> avez configurés.
</p>
<p>
  Le site compagnon ferrflow.com n'utilise aucun cookie ni traceur tiers ; voir la
  <a href="/fr/cookies">page cookies</a> et la
  <a href="/fr/privacy">politique de confidentialité</a> pour le détail complet.
</p>

<h2>Contact</h2>
<p>
  <a href="mailto:security@ferrlabs.com">security&#64;ferrlabs.com</a> ·
  <a href="https://github.com/FerrLabs/FerrFlow/security/advisories/new">Advisory GitHub</a> ·
  <a href="https://ferrlabs.com/fr/security">Sécurité plateforme FerrLabs →</a>
</p>
