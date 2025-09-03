#!/usr/bin/env node
import { rmSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const root = process.cwd();

// List of relative paths to remove
const pathsToRemove = [
  'node_modules', // SDK root dependencies
  join('core', 'node_modules'),
  join('msp-client', 'node_modules'),
  // build artifacts
  join('core', 'dist'),
  join('msp-client', 'dist'),
  join('core', 'coverage'),
  join('msp-client', 'coverage'),
  join('core', 'wasm', 'target'), // Cargo build artifacts (if present)
];

pathsToRemove.forEach((relativePath) => {
  const absPath = join(root, relativePath);
  if (existsSync(absPath)) {
    console.log(`Removing ${absPath}`);
    rmSync(absPath, { recursive: true, force: true });
  }
});

// Remove files inside core/wasm/pkg but keep the directory itself
const pkgDir = join(root, 'core', 'wasm', 'pkg');
if (existsSync(pkgDir)) {
  const { readdirSync } = await import('node:fs');
  for (const entry of readdirSync(pkgDir)) {
    // Preserve hidden files such as .gitkeep
    if (entry.startsWith('.')) continue;
    const entryPath = join(pkgDir, entry);
    rmSync(entryPath, { recursive: true, force: true });
  }
  console.log(`Cleaned contents of ${pkgDir}`);
}

console.log('Cleanup completed.'); 