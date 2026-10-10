---
title: 'FerrFlow: docs in the Claude style'
summary: 'ferrflow.com/docs/ adopts the visual language of docs.claude.com: off-white background, FerrFlow orange accent, soft-dark code blocks, accent-bar callouts, a 3-column layout and <Card>/<CardGroup>/<Note> components.'
date: 2026-04-29T16:30:00Z
product: 'ferrflow'
type: 'new'
---

The default Starlight docs did their job but looked generic. They now borrow the visual language of the Anthropic docs, keeping FerrFlow orange as the single accent instead of Anthropic's clay.

What changes visually:

- **Off-white `#fafafa` background** (light) and **near-black `#0a0a0a`** (dark), with zinc text.
- **Inter system stack** for body text, Fraunces kept for headings.
- **FerrFlow orange accent `#e8733a`** (light) and `#ef8a55` (dark) for links, focus rings and the active sidebar item.
- **Soft-dark code blocks `#1c1c1c`** even in light mode, 12px rounded, language label top-left, copy button top-right on hover.
- **Tinted-card callouts** with a 4px accent bar on the left: `<Note>`, `<Tip>`, `<Warning>`, `<Info>`.
- **Mintlify-style components** `<Card>` and `<CardGroup cols={2|3}>` for value grids.
- **Small-caps breadcrumb** "Section › Page" above the H1.
- **"Was this page helpful?" widget** and an "Edit on GitHub" link at the bottom of every page.

Layout, version and language sidebar, ExpressiveCode, FR translations and the `v0`/`v1`/`v2`/`v3` snapshots inherit it automatically since the CSS is global. Pagefind search is unchanged. The Cmd+K modal keeps its Starlight style for now.
