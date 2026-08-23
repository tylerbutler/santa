# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Rust developers who need to parse, navigate, serialize, or round-trip CCL configuration in their applications and tools.

## Product Purpose

Sickle provides a robust Rust implementation of CCL (Categorical Configuration Language). It lets developers work directly with a CCL object model or integrate CCL with typed Rust data through Serde. Success means developers can adopt only the CCL capabilities they need while retaining predictable parsing and serialization behavior.

## Positioning

Sickle combines feature-gated CCL parsing with two complementary API styles: direct model navigation and Serde-based typed data. Unlike a generic configuration parser, its data model, hierarchy handling, printer, and document-editing capabilities are built specifically around CCL semantics.

## Operating Context

Sickle is an independently published crates.io library and the foundational CCL layer for the Santa workspace. Developers consume it as a Cargo dependency, select capabilities through feature flags, and use its parsing, hierarchy, Serde, printer, or document APIs in Rust applications and tooling.

## Capabilities and Constraints

- Parse CCL into flat entries or a hierarchical object model.
- Deserialize CCL into Rust types and serialize Rust values to CCL through optional Serde features.
- Print canonical CCL and round-trip typed edits while preserving document comments and formatting through the optional document API.
- Preserve insertion order by default; reference-implementation duplicate-key ordering is explicitly opt-in.
- Remain pure Rust with no unsafe code.
- Keep optional capabilities and their dependencies behind granular Cargo feature flags; the default feature set remains empty.
- Maintain Sickle as a general-purpose library with no dependency on Santa.

## Brand Commitments

The product name is **Sickle**. Product communication should be technical, direct, and evidence-based. Do not imply that Sickle is the CCL reference implementation; reference compatibility is an optional mode.

## Evidence on Hand

- Public package metadata and feature contract: `Cargo.toml`
- Current product overview, syntax examples, and API examples: `README.md`
- Public API and implementation: `src/`
- Working direct and Serde examples: `examples/`
- Parser, integration, property, and data-driven test evidence: `tests/`
- Workspace architecture and independent-publication role: `../../ARCHITECTURE.md`
- No testimonials, customer logos, adoption metrics, benchmarks, or visual brand assets are present; future work must not fabricate them.

## Product Principles

1. Let developers pay only for the CCL capabilities they use.
2. Make direct model access and typed Serde workflows equally intentional.
3. Preserve CCL semantics and source fidelity rather than flattening the language into generic configuration behavior.
4. Keep the core safe, dependency-conscious, and independent of Santa.
5. Demonstrate claims with executable examples and tests.
