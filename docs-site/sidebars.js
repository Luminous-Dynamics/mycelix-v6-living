// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  docs: [
    'intro',
    'installation',
    'quick-start',
    {
      type: 'category',
      label: 'Concepts',
      collapsed: false,
      items: [
        'concepts/cycle',
        'concepts/phases',
        'concepts/primitives',
      ],
    },
    {
      type: 'category',
      label: 'Server',
      items: [
        'server/configuration',
        'server/deployment',
        'server/security',
      ],
    },
    {
      type: 'category',
      label: 'API Reference',
      items: [
        'api/websocket',
        'api/rest',
        'api/graphql',
      ],
    },
    {
      type: 'category',
      label: 'SDKs',
      items: [
        'sdks/typescript',
        'sdks/python',
        'sdks/go',
      ],
    },
    'contributing',
  ],
};

export default sidebars;
