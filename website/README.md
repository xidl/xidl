# XIDL Documentation Website

This is the new documentation site for XIDL, built with
[Starlight](https://starlight.astro.build/).

## Project Structure

- `src/content/docs/`: Main documentation content (English, root `/`).
- `public/`: Static assets.

## Workflow

1. **Write content**: New content should be authored in `src/content/docs/`.
2. **Regenerate the changelog page**: `pnpm build` runs
   `node scripts/copy-changelog.mjs`, which mirrors `CHANGELOG.md` into
   `src/content/docs/changelog.md`.

## Commands

- `pnpm dev`: Start local development server.
- `pnpm build`: Build for production.
- `pnpm format`: Format code with Biome.
- `pnpm lint`: Lint code with Biome.
