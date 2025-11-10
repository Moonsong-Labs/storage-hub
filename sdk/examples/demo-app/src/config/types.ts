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


