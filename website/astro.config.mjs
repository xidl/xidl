import netlify from '@astrojs/netlify';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';
import idlLanguage from './src/shiki/idl.mjs';

// https://astro.build/config
export default defineConfig({
  adapter: netlify(),
  base: '/',

  integrations: [
    starlight({
      components: {
        SiteTitle: './src/components/SiteTitle.astro',
      },
      customCss: ['./src/styles/global.css'],
      defaultLocale: 'root',
      expressiveCode: {
        shiki: {
          langAlias: {
            xidl: 'idl',
          },
          langs: [idlLanguage],
        },
      },
      head: [
        {
          attrs: {
            'data-library': '/xidl/xidl',
            defer: true,
            src: 'https://context7.com/widget.js',
          },
          tag: 'script',
        },
        ...(process.env.GOOGLE_ANALYTICS
          ? [
              {
                attrs: {
                  async: true,
                  src: `https://www.googletagmanager.com/gtag/js?id=${process.env.GOOGLE_ANALYTICS}`,
                },
                tag: 'script',
              },
              {
                content: `
                  window.dataLayer = window.dataLayer || [];
                  function gtag(){dataLayer.push(arguments);}
                  gtag('js', new Date());
                  gtag('config', '${process.env.GOOGLE_ANALYTICS}');
                `,
                tag: 'script',
              },
            ]
          : []),
      ],
      locales: {
        root: {
          label: 'English',
          lang: 'en',
        },
      },
      sidebar: [
        {
          items: [
            {
              label: 'Install',
              link: '/guide/',
            },
            {
              label: 'First HTTP API',
              link: '/guide/first-http-api/',
            },
            {
              label: 'Editor',
              link: '/guide/editor/',
            },
            {
              label: 'Rust Integration',
              link: '/guide/rust-integration/',
            },
          ],
          label: 'Guide',
        },
        {
          autogenerate: { directory: 'docs' },
          label: 'Docs',
        },
        {
          autogenerate: { directory: 'rest' },
          label: 'xidl_for_rest',
        },
        {
          autogenerate: { directory: 'jsonrpc' },
          label: 'xidl_for_jsonrpc',
        },
        {
          autogenerate: { directory: 'rfc' },
          label: 'RFC',
        },
        {
          autogenerate: { directory: 'ai' },
          label: 'AI',
        },
      ],
      social: [
        {
          href: 'https://github.com/loongtao/xidl',
          icon: 'github',
          label: 'GitHub',
        },
      ],
      title: 'XIDL',
    }),
  ],

  site: 'https://xidl.netlify.app/',

  vite: {
    plugins: [tailwindcss()],
  },
});
