declare global {
  // Allow process.env usage in the browser build for env-inlined values
  namespace NodeJS {
    interface ProcessEnv {
      NEXT_PUBLIC_APP_CONFIG_FILE?: string;
    }
  }
  const process: { env: NodeJS.ProcessEnv };
  
  interface Window {
    ethereum?: EthereumProvider | EthereumProvider[];
  }

  interface EthereumProvider {
    request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
    on: (event: string, handler: (...args: unknown[]) => void) => void;
    removeListener?: (event: string, handler: (...args: unknown[]) => void) => void;
    isMetaMask?: boolean;
    isCoinbaseWallet?: boolean;
    isBraveWallet?: boolean;
    providers?: EthereumProvider[];
    selectedAddress?: string;
    chainId?: string;
    _metamask?: {
      isUnlocked?: () => Promise<boolean>;
    };
  }
}

export { };
