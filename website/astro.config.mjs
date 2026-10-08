import netlify from '@astrojs/netlify';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';
import starlightLlmsTxt from 'starlight-llms-txt';
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
      plugins: [starlightLlmsTxt()],
      sidebar: [
        {
          items: [
            {
              label: 'Install & Quickstart',
              link: '/guide/',
            },
            {
              label: 'First HTTP API',
              link: '/guide/first-http-api/',
            },
            {
              label: 'Rust Integration',
              link: '/guide/rust-integration/',
            },
            {
              label: 'Editor',
              link: '/guide/editor/',
            },
            {
              label: 'Language Support',
              link: '/guide/language-support/',
            },
          ],
          label: 'Guide',
        },
        {
          items: [{ autogenerate: { directory: 'docs' } }],
          label: 'Reference',
        },
        {
          items: [{ autogenerate: { directory: 'rest' } }],
          label: 'HTTP & REST',
        },
        {
          items: [{ autogenerate: { directory: 'jsonrpc' } }],
          label: 'JSON-RPC',
        },
        {
          items: [{ autogenerate: { directory: 'rfc' } }],
          label: 'RFC',
        },
        {
          items: [{ autogenerate: { directory: 'ai' } }],
          label: 'AI',
        },
        {
          label: 'Changelog',
          link: '/changelog/',
        },
      ],
      social: [
        {
          href: 'https://github.com/xidl/xidl',
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
