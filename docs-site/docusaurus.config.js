// @ts-check
import { themes as prismThemes } from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Mycelix',
  tagline: 'A Living System for Distributed Intelligence',
  favicon: 'img/favicon.ico',

  url: 'https://mycelix.io',
  baseUrl: '/',

  organizationName: 'mycelix',
  projectName: 'mycelix',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl: 'https://github.com/mycelix/mycelix/tree/main/docs-site/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/mycelix-social-card.png',
      navbar: {
        title: 'Mycelix',
        logo: {
          alt: 'Mycelix Logo',
          src: 'img/logo.svg',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docs',
            position: 'left',
            label: 'Documentation',
          },
          {
            href: '/docs/api/websocket',
            label: 'API Reference',
            position: 'left',
          },
          {
            href: 'https://github.com/mycelix/mycelix',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              { label: 'Getting Started', to: '/docs/intro' },
              { label: 'Installation', to: '/docs/installation' },
              { label: 'Quick Start', to: '/docs/quick-start' },
            ],
          },
          {
            title: 'Concepts',
            items: [
              { label: 'The 28-Day Cycle', to: '/docs/concepts/cycle' },
              { label: 'Phases', to: '/docs/concepts/phases' },
              { label: '21 Primitives', to: '/docs/concepts/primitives' },
            ],
          },
          {
            title: 'SDKs',
            items: [
              { label: 'TypeScript', to: '/docs/sdks/typescript' },
              { label: 'Python', to: '/docs/sdks/python' },
              { label: 'Go', to: '/docs/sdks/go' },
            ],
          },
          {
            title: 'More',
            items: [
              { label: 'GitHub', href: 'https://github.com/mycelix/mycelix' },
              { label: 'Contributing', to: '/docs/contributing' },
            ],
          },
        ],
        copyright: `Copyright ${new Date().getFullYear()} Mycelix Project. Built with Docusaurus.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['bash', 'json', 'yaml', 'typescript', 'python', 'go', 'graphql'],
      },
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
    }),
};

export default config;
