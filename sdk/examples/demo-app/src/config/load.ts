import type { AppConfig } from './types';

export async function loadAppConfig(url?: string): Promise<AppConfig> {
  // Prefer explicit arg; otherwise rely on env var.
  let resolvedUrl = url;
  if (!resolvedUrl) {
    const env = (globalThis as unknown as { process?: { env?: Record<string, string | undefined> } }).process?.env;
    resolvedUrl = env?.APP_CONFIG_FILE || env?.NEXT_PUBLIC_APP_CONFIG_FILE;
  }
  if (!resolvedUrl) {
    throw new Error('App config not specified. Set APP_CONFIG_FILE to /storagehub.local.json or /storagehub.stagenet.json');
  }

  const configUrl = resolvedUrl;
  // Debug: which URL is being used
  try { console.info('[AppConfig] Loading config from:', configUrl); } catch { }
  const res = await fetch(configUrl, { cache: 'no-store' });
  if (!res.ok) {
    throw new Error(`Failed to load config: ${res.status} ${res.statusText}`);
  }
  const json = (await res.json()) as unknown;
  try { console.info('[AppConfig] Loaded config JSON:', json); } catch { }
  return json as AppConfig;
}


