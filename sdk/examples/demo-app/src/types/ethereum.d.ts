interface Window {
  ethereum?: EthereumProvider;
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
