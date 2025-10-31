import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // Silence workspace root warning and align Turbopack root to monorepo root
  turbopack: {
    root: path.join(__dirname, '../../..'),
  },
};

export default nextConfig;
