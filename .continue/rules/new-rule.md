---
name: Bevy 0.16: Authoring & Review Rules
description: Enforce Bevy 0.16 docs as the source of truth, require full-file outputs, and encourage ECS separation of concerns.
globs: ["**/*.rs", "Cargo.toml", "crates/**"]
# If your Continue supports regex matching, uncomment the next line to auto-apply when using Bevy 0.16:
# regex: 'bevy\s*=\s*"?0\.16'
alwaysApply: true
---

# Bevy 0.16 Rule Set

You must follow all of the rules below when generating, editing, or reviewing code for this repository.

## 1) Source of truth: Bevy 0.16 documentation
- Treat the official Bevy 0.16 docs and guides as canonical references.
- Prefer API names, patterns, and migration notes that match Bevy 0.16 specifically (not older versions).
- When you rely on an API or pattern that changed in 0.16, mention the relevant section from the docs or migration guide inline in your explanation comments.

Authoritative references:
- Bevy 0.16 release notes and docs hub.
- Migration guide 0.15 → 0.16.

## 2) Output format: print whole files, not diffs
- When proposing changes, always output the full content of each affected file.
- Do not use “diff”, “patch”, or “inline change” formats.
- For each file, use a separate fenced code block with the correct language tag.
- Start the code block with an inline comment containing the file path, for example:
  - Rust source: use `// path: path/to/file.rs` as the first line.
  - TOML: use `# path: Cargo.toml` as the first line.
- If multiple files change, output multiple full-file code blocks, one after another.
- After the code blocks, include a concise summary of what changed and why.

## 3) Bevy ECS best practices: separation of concerns
Structure all gameplay and engine integration code to reflect ECS principles:

- Components:
  - Data only; no behavior. Keep them small, serializable when reasonable, and named clearly.
  - Prefer immutable data by default; only use `mut` where necessary.
  - Group commonly co-instantiated components in Bundles.

- Systems:
  - Single-responsibility systems. One clear job per system.
  - Keep system function signatures minimal; prefer specific queries over broad ones.
  - Use Events to decouple producers and consumers of cross-cutting logic.
  - Use System Sets to order related systems and to gate them behind run conditions.
  - Minimize global mutable state; prefer Resources for explicit, shared state when needed.

- Plugins and modularity:
  - Organize features as Plugins that register components, events, systems, and schedules.
  - Keep rendering, input, physics, UI, and gameplay logic in separate plugins/modules.
  - Provide a top-level “game” plugin that composes feature plugins.

- Scheduling:
  - Place initialization in Startup systems.
  - Use run conditions and explicit system ordering where correctness depends on it.

- Command and Query usage:
  - Use `Commands` for entity lifecycle (spawn/despawn/insert/remove).
  - Keep queries tight: prefer `Query<(&A, &mut B), (With<C>, Without<D>)>`-style filters to avoid unnecessary work.

- Testing and examples:
  - Prefer minimal, focused examples/tests per component or system.
  - Avoid test flakiness by controlling schedules and seeding any randomness.

## 4) When in doubt, clarify with a short checklist
Before finalizing output:
- [ ] Did you consult or align with Bevy 0.16 docs/migration notes for any API you used?
- [ ] Did you print full files (each in its own fenced code block with a path comment)?
- [ ] Are components data-only and systems single-responsibility?
- [ ] Are plugins used to compose features cleanly?
- [ ] Are queries minimal and appropriately filtered?
