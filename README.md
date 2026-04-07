# neco editor

[日本語](README-ja.md)

`neco editor` is a set of Rust crates for text editing, file tree management, and file watcher event normalization in editor runtimes.

This repository collects editor-side primitives that were factored out of an application codebase into independently publishable crates. Each crate handles one narrow concern — path relation logic, in-memory file trees, small text patches, or watcher event coalescing — so they can be consumed separately on crates.io.

More crates may be added over time.

## Crates

| Crate | Summary | Internal dependencies | Main external dependencies |
|---|---|---|---|
| [`neco-pathrel`](./neco-pathrel) | string-based path relation and rename remap helpers | none | none |
| [`neco-filetree`](./neco-filetree) | pure file tree lookup, merge, flatten, and reveal helpers | `neco-pathrel` | none |
| [`neco-textpatch`](./neco-textpatch) | deterministic narrow text patch helpers for small structured edits | none | none |
| [`neco-watchnorm`](./neco-watchnorm) | host-agnostic file watcher event normalization and batch coalescing | none | none |
| [`neco-textview`](./neco-textview) | line-indexed text buffer with efficient position/offset conversion | none | none |
| [`neco-decor`](./neco-decor) | span-based decoration model for editor overlays | `neco-textview` | none |
| [`neco-diffcore`](./neco-diffcore) | minimal diff engine for line-level change detection | none | none |
| [`neco-wrap`](./neco-wrap) | soft-wrap line map for monospace editors | `neco-textview` | none |

Each crate is intentionally independent so it can be published and consumed separately on crates.io. The repository is a monorepo for maintenance convenience, not a runtime-coupled framework.

This repository is still under active development, and crates vary in maturity. Some parts are already usable, while others are still being hardened or reshaped.

Updates may still change internal implementations relatively often. In particular, algorithm swaps and performance-oriented rewrites are more likely than long-term API stability across every crate.

## Status

- Workspace formatting, lint, and test gates are maintained at the repository level.
- GitHub Actions CI is configured in [`.github/workflows/ci.yml`](./.github/workflows/ci.yml).
- Individual crates may evolve at different speeds.

## Contribution

Issues and pull requests are welcome. In practice, focused requests with a clear target are easier to review and validate than broad or vague proposals.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development workflow and [SECURITY.md](./SECURITY.md) for security reporting.

## Support

If these crates or related apps are useful to you, you can support ongoing development here:

- OFUSE: <https://ofuse.me/barineco>
- Ko-fi: <https://ko-fi.com/barineco>

Support helps sustain maintenance, documentation, and ongoing development.

## License

Unless noted otherwise, this repository is licensed under the [MIT License](./LICENSE).
