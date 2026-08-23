---
version: 1
slug: "website-src-pages-index-astro"
primary_target: "website/src/pages/index.astro"
related_targets: []
---

## Scope and mode

- Surface: `website/src/pages/index.astro`
- Mode: Persuade
- Implementation: Astro static site

## Audience and job

Rust developers evaluating whether Sickle is the right CCL parser. They need to understand the API shape, choose a feature set, and add the crate to a project without hunting through marketing copy.

## Action and proof

- Primary action: copy the Cargo add command.
- Secondary actions: inspect the direct-model and Serde APIs, then open docs.rs or the repository.
- Proof must come from real syntax, Cargo feature definitions, pure-Rust/no-unsafe constraints, source-preserving document support, and executable examples already in the crate.
- Do not invent adoption metrics, benchmarks, customers, testimonials, or compatibility claims.

## Direction

Compiler Diagnostic Sheet: one continuous annotated source artifact turns CCL input into direct-model access, Serde output, and exact feature instructions. The memorable moment is the first viewport's oversized source listing, where annotations resolve into an immediately copyable Cargo command.

## Constraints

- Avoid generic neon developer-tool styling and a conventional split hero plus feature-card grid.
- Technical trust and code readability outrank decorative nostalgia.
- Keyboard access, reduced motion, mobile code legibility, and clear focus states are required.
- The site must remain a static, dependency-light Astro build.

## Unresolved decisions

- Deployment target is not yet specified.
