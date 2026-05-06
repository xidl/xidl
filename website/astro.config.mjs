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
              label: 'Install',
              link: '/guide/',
              translations: {
                'zh-CN': '安装',
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
          translations: {
            'zh-CN': 'IDL 语言',
          },
        },
        {
          autogenerate: { directory: 'rest' },
          label: 'xidl_for_rest',
          translations: {
            'zh-CN': 'XIDL REST 支持',
          },
        },
        {
          autogenerate: { directory: 'rfc' },
          label: 'RFC',
        },
        {
          autogenerate: { directory: 'development' },
          label: 'Development',
          translations: {
            'zh-CN': '开发文档',
          },
        },
        {
          autogenerate: { directory: 'ai' },
          label: 'AI',
          translations: {
            'zh-CN': 'AI 技能',
          },
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
