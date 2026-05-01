import netlify from '@astrojs/netlify';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

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
            {
              label: 'Quickstart',
              link: '/guide/',
              translations: {
                'zh-CN': '快速开始',
              },
            },
            {
              label: 'First HTTP API',
              link: '/guide/first-http-api/',
              translations: {
                'zh-CN': '第一个 HTTP API',
              },
            },
            {
              label: 'Editor',
              link: '/guide/editor/',
              translations: {
                'zh-CN': '编辑器',
              },
            },
          ],
          label: 'Guide',
          translations: {
            'zh-CN': '指南',
          },
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
});
