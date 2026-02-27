const sidebars = {
  docs: [
    {
      type: 'doc',
      id: 'index',
      label: 'Home',
    },
    {
      type: 'doc',
      id: 'plugin',
      label: 'Plugin Development',
    },
    {
      type: 'category',
      label: 'xidlc',
      items: [
        {
          type: 'doc',
          id: 'rust-axum',
          label: 'axum',
        },
        {
          type: 'doc',
          id: 'rust-jsonrpc',
          label: 'jsonrpc',
        },
      ],
    },
  ],
};

export default sidebars;
