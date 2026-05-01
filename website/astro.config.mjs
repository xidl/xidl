import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

import netlify from '@astrojs/netlify';

// https://astro.build/config
export default defineConfig({
  base: '/',

  integrations: [
    starlight({
      components: {
        SiteTitle: './src/components/SiteTitle.astro',
      },
      customCss: ['./src/styles/global.css'],
      defaultLocale: 'root',
      locales: {
        root: {
          label: 'English',
          lang: 'en',
        },
        'zh-cn': {
          label: '简体中文',
          lang: 'zh-CN',
        },
      },
      sidebar: [
        {
          items: [
            { label: 'Overview', link: '/guide/' },
            { label: 'Quickstart', link: '/guide/quickstart/' },
            { label: 'First HTTP API', link: '/guide/first-http-api/' },
            { label: 'First Rust Project', link: '/guide/first-rust-project/' },
          ],
          label: 'Guide',
        },
        {
          autogenerate: { directory: 'docs' },
          label: 'Docs',
        },
        {
          autogenerate: { directory: 'rfc' },
          label: 'RFC',
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

  adapter: netlify(),
});
