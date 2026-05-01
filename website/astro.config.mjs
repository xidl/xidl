import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
	site: 'https://xidl.github.io',
	base: '/xidl/',
	integrations: [      starlight({
          title: 'XIDL',
          customCss: ['./src/styles/global.css'],
          defaultLocale: 'root',          locales: {
              root: {
                  label: 'English',
                  lang: 'en',
              },
              'zh-cn': {
                  label: '简体中文',
                  lang: 'zh-CN',
              },
          },
          social: [
              {
                  icon: 'github',
                  label: 'GitHub',
                  href: 'https://github.com/loongtao/xidl',
              },
          ],
          components: {
              SiteTitle: './src/components/SiteTitle.astro',
          },
          sidebar: [
              {
                  label: 'Guide',
                  items: [
                      { label: 'Overview', link: '/guide/' },
                      { label: 'Quickstart', link: '/guide/quickstart/' },
                      { label: 'First HTTP API', link: '/guide/first-http-api/' },
                      { label: 'First Rust Project', link: '/guide/first-rust-project/' },
                  ],
              },
              {
                  label: 'Docs',
                  autogenerate: { directory: 'docs' },
              },
              {
                  label: 'RFC',
                  autogenerate: { directory: 'rfc' },
              },
          ],
      }),
	],

  vite: {
    plugins: [tailwindcss()],
  },
});