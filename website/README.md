# XIDL Documentation Website

This is the new documentation site for XIDL, built with
[Starlight](https://starlight.astro.build/).

## Project Structure

- `src/content/docs/`: Main documentation content.
  - `en/`: English documentation (Default locale, root `/`).
  - `zh-cn/`: Chinese documentation (Locale `/zh-cn/`).
- `public/`: Static assets.

## Workflow

1. **Write in Chinese**: New content should be authored first in
   `src/content/docs/zh-cn/`.
2. **Translate to English**: Use AI to translate the Chinese content into
   English and place it in the corresponding `src/content/docs/en/` path.
3. **Update Status**: Maintain the `status` and `translationStatus` in the
   frontmatter.

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
