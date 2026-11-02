import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { scalarStarlight } from '@scalar/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';
import idlLanguage from './src/shiki/idl.mjs';
import publication from './xidl-site.json' with { type: 'json' };

export default defineConfig({
  base: publication.base,
  integrations: [
    starlight({
      components: { Head: './src/components/Head.astro' },
      customCss: ['./src/styles/global.css'],
      expressiveCode: {
        frames: {
          extractFileNameFromCode: false,
          removeCommentsWhenCopyingTerminalFrames: false,
        },
        shiki: { langs: [idlLanguage] },
        themes: ['github-light', 'github-dark'],
      },
      plugins: publication.playgrounds.map(reference =>
        scalarStarlight({
          configuration: {
            agent: { disabled: true },
            cdn: `${publication.base}_scalar/scalar.js`,
            hideDarkModeToggle: true,
            persistAuth: false,
            proxyUrl: '',
            showDeveloperTools: 'never',
            telemetry: false,
            url: `${publication.base}${reference.specPath}`,
            withDefaultFonts: false,
          },
          label: reference.title,
          pathname: reference.pathname,
          title: reference.title,
        }),
      ),
      routeMiddleware: './src/route-data.ts',
      sidebar: publication.sidebar,
      title: publication.title,
    }),
  ],
  markdown: {
    processor: satteri({
      features: { headingAttributes: true, smartPunctuation: false },
    }),
  },
  site: publication.site ?? undefined,
  trailingSlash: 'always',
  vite: { plugins: [tailwindcss()] },
});
