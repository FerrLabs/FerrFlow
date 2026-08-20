---
title: 'FerrFlow · docs au style Claude'
summary: 'ferrflow.com/docs/ adopte le langage visuel de docs.claude.com — fond off-white, accent FerrFlow orange, code blocks soft-dark, callouts à barre accent, layout 3 colonnes, composants <Card>/<CardGroup>/<Note>.'
date: 2026-04-29T16:30:00Z
product: 'ferrflow'
type: 'new'
---

La doc Starlight par défaut faisait son boulot mais avait l'air générique. On adopte le langage visuel des docs Anthropic, en gardant l'orange FerrFlow comme accent unique au lieu de la clay Anthropic.

Ce qui change visuellement :

- **Fond off-white `#fafafa`** (light) / **near-black `#0a0a0a`** (dark) avec texte zinc.
- **Inter system stack** pour le corps, Fraunces gardé pour les titres.
- **Accent FerrFlow orange `#e8733a`** (light) / `#ef8a55` (dark) pour les liens, focus rings, sidebar active item.
- **Code blocks soft-dark `#1c1c1c`** même en light mode, 12px rounded, label langue top-left, bouton copy top-right au hover.
- **Callouts à carte teintée** avec barre accent 4px à gauche : `<Note>`, `<Tip>`, `<Warning>`, `<Info>`.
- **Composants Mintlify-style** `<Card>` et `<CardGroup cols={2|3}>` pour les grilles de valeur.
- **Breadcrumb small-caps** "Section › Page" au-dessus du H1.
- **Widget "Was this page helpful?"** + lien "Edit on GitHub" en bas de chaque page.

Layout, sidebar versions/langues, ExpressiveCode, FR translations et snapshots `v0/v1/v2/v3` héritent automatiquement (le CSS est global). Pagefind search inchangée — la modale Cmd+K garde son style Starlight pour l'instant.
