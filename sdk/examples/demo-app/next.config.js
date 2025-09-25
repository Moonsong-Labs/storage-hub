const path = require('path');

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // Silence workspace root warning and align Turbopack root to monorepo root
  turbopack: {
    root: path.join(__dirname, '../../..'),
  },
};

module.exports = nextConfig;
