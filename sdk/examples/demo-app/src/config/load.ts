import type { AppConfig } from './types';

export async function loadAppConfig(url?: string): Promise<AppConfig> {
  // Prefer explicit arg; otherwise rely on env var.
  let resolvedUrl = url;
  if (!resolvedUrl) {
    // In Next.js, process.env values are replaced at build time
    // For client-side, only NEXT_PUBLIC_* vars are available
    resolvedUrl = process.env.NEXT_PUBLIC_APP_CONFIG_FILE;
  }
  if (!resolvedUrl) {
    throw new Error('App config not specified. Set NEXT_PUBLIC_APP_CONFIG_FILE to /storagehub.local.json or /storagehub.stagenet.json');
  }

  const configUrl = resolvedUrl;
  // Debug: which URL is being used
  console.info('[AppConfig] Loading config from:', configUrl);
  console.info('[AppConfig] NEXT_PUBLIC_APP_CONFIG_FILE:', process.env.NEXT_PUBLIC_APP_CONFIG_FILE);
  const res = await fetch(configUrl, { cache: 'no-store' });
  if (!res.ok) {
    throw new Error(`Failed to load config: ${res.status} ${res.statusText}`);
  }
  const json = (await res.json()) as unknown;
  try { console.info('[AppConfig] Loaded config JSON:', json); } catch { }
  return json as AppConfig;
}


