# XIDL Documentation Website

This is the new documentation site for XIDL, built with
[Starlight](https://starlight.astro.build/).

## Project Structure

- `src/content/docs/`: Main documentation content (English, root `/`).
- `public/`: Static assets.

## Workflow

1. **Write content**: New content should be authored in `src/content/docs/`.
2. **Maintain Status**: Keep the `status` and `translationStatus` in the
   frontmatter up to date.

## Frontmatter Fields

### Content Status (`status`)

- `planned`: Page exists as a placeholder.
- `draft`: Initial content is being written.
- `reviewing`: Content is complete but needs technical review.
- `published`: Content is final and accurate.

### Translation Status (`translationStatus`)

- `none`: No translation exists.
- `machine-draft`: Initial AI-generated translation.
- `needs-review`: Translation exists but needs human check.
- `reviewed`: Translation is verified and accurate.

## Commands

- `pnpm dev`: Start local development server.
- `pnpm build`: Build for production.
- `pnpm format`: Format code with Biome.
- `pnpm lint`: Lint code with Biome.
