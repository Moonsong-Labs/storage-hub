export interface AppConfig {
  chain: {
    id: number;
    name: string;
    nativeCurrency: { name: string; symbol: string; decimals: number };
    evmRpcHttpUrl: string;
    substrateRpcWsUrl?: string;
    blockExplorerUrl?: string;
    filesystemPrecompileAddress?: `0x${string}`;
  };
  msp: {
    baseUrl: string;
    timeoutMs?: number;
    headers?: Record<string, string>;
  };
  auth: {
    /** SIWE domain (must match the site host, e.g., "localhost:3001" or "datahaven.app") */
    siweDomain: string;
    /** Public URL of this web app for the SIWE "URI:" field (e.g., "https://localhost:3001") */
    siweUri: string;
  };
  defaults?: {
    replicationLevel?: 'Basic' | 'Standard' | 'Premium' | 'Custom';
    replicas?: number;
    gas?: string; // decimal string for BigInt
    gasPriceWei?: string; // decimal string for BigInt
    delays?: {
      postStorageRequestMs?: number;
      beforeUploadMs?: number;
    };
  };
}


