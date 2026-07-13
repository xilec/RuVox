# Decisions

Short rationale log for non-obvious architectural/tooling choices that aren't derivable from the code itself.

## Package manager: pnpm (not bun)

pnpm is the frontend package manager. The choice was between pnpm and bun; pnpm won primarily because of **Nix build reproducibility**, not because of monorepo/workspace features (the repo has a single `package.json`, no workspaces).

Production builds go through a reproducible Nix flake (`flake.nix` → `.#ruvox`), which pins frontend deps via `pnpm.fetchDeps` and a fixed-output-derivation hash (see the "First `nix build` run" note in [development.md](development.md)). Reasons pnpm fits that path better than bun:

- **Nix packaging maturity** — `pnpm2nix`/`pnpm.fetchDeps` is mainstream in nixpkgs; bun's Nix packaging (`bun2nix` and similar) is newer and less battle-tested, historically complicated by the binary `bun.lockb` lockfile.
- **Lockfile format** — pnpm's `pnpm-lock.yaml` is a diffable text file, easy to hash for a fixed-output derivation.
- **Runtime risk** — bun replaces the Node runtime itself, not just the package manager. For a Tauri + Vite stack with native bindings, that's a bigger surface for incompatibility than a pure package-manager swap.
- **Install speed** — bun is faster at installing than pnpm, but Nix already caches dependencies in the store, so this advantage matters less here.
- **Strictness** — pnpm's non-flat `node_modules` prevents phantom-dependency bugs; bun uses a flat layout like npm/yarn.
