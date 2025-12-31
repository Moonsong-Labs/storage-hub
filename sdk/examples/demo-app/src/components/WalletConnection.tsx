'use client';

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Wallet, AlertCircle, ExternalLink, Copy, CheckCircle } from 'lucide-react';
import { createWalletClient, createPublicClient, custom, defineChain, formatEther, type WalletClient, type PublicClient } from 'viem';
import { loadAppConfig } from '../config/load';
import type { AppConfig } from '../config/types';
import { getMetaMaskProvider, safeProviderRequest } from '../utils/metamask';

interface WalletConnectionProps {
  onWalletConnected: (connected: boolean) => void;
  walletConnected: boolean;
  configurationValid: boolean;
  onClientsReady?: (clients: { walletClient: WalletClient; publicClient: PublicClient } | null) => void;
  onAddressChange?: (address: string | null) => void;
}

interface WalletState {
  address: string | null;
  balance: string | null;
  chainId: number | null;
  isConnecting: boolean;
  error: string | null;
}

export function WalletConnection({
  onWalletConnected,
  walletConnected,
  configurationValid,
  onClientsReady,
  onAddressChange
}: WalletConnectionProps) {
  const [walletState, setWalletState] = useState<WalletState>({
    address: null,
    balance: null,
    chainId: null,
    isConnecting: false,
    error: null
  });
  const [copied, setCopied] = useState(false);

  // MetaMask availability checks are done inline with window?.ethereum

  // Load app config and derive expected network
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  useEffect(() => {
    const run = async () => {
      try {
        const cfg = await loadAppConfig();
        setAppConfig(cfg);
      } catch (e) {
        console.warn('Failed to load app config in WalletConnection; using defaults', e);
      }
    };
    void run();
  }, []);

  const expectedChainId = appConfig?.chain.id ?? 181222;
  const expectedRpcUrl = appConfig?.chain.evmRpcHttpUrl ?? 'http://127.0.0.1:9888';

  // Define StorageHub chain from config (fallback to local defaults)
  const storageHubChain = useMemo(() => defineChain({
    id: expectedChainId,
    name: appConfig?.chain.name ?? 'StorageHub Solochain EVM',
    nativeCurrency: appConfig?.chain.nativeCurrency ?? { name: 'StorageHub', symbol: 'SH', decimals: 18 },
    rpcUrls: { default: { http: [expectedRpcUrl] } }
  }), [appConfig, expectedChainId, expectedRpcUrl]);

  // Switch to StorageHub network
  const switchToStorageHubNetwork = useCallback(async () => {
    const provider = getMetaMaskProvider();
    if (!provider) return false;

    try {
      console.log('Attempting to switch to StorageHub network...');
      // Try to switch to the network first
      await safeProviderRequest(provider, 'wallet_switchEthereumChain', [{ chainId: `0x${expectedChainId.toString(16)}` }]);
      console.log('Successfully switched to existing network');
      return true;
    } catch (switchError: unknown) {
      console.log('Switch failed, checking error:', switchError);
      // If network doesn't exist (error 4902), add it
      if ((switchError as { code?: number }).code === 4902) {
        console.log('Network not found, attempting to add...');
        try {
          await safeProviderRequest(provider, 'wallet_addEthereumChain', [{
            chainId: `0x${expectedChainId.toString(16)}`,
            chainName: appConfig?.chain.name ?? 'StorageHub Solochain EVM',
            nativeCurrency: appConfig?.chain.nativeCurrency ?? { name: 'StorageHub', symbol: 'SH', decimals: 18 },
            rpcUrls: [expectedRpcUrl],
            blockExplorerUrls: null
          }]);
          console.log('Successfully added and switched to new network');
          return true;
        } catch (addError) {
          console.error('Failed to add StorageHub network:', addError);
          return false;
        }
      } else {
        console.error('Failed to switch to StorageHub network:', switchError);
        return false;
      }
    }
  }, [appConfig, expectedChainId, expectedRpcUrl]);

  // Get chain ID
  const getChainId = useCallback(async (): Promise<number> => {
    try {
      const provider = getMetaMaskProvider();
      if (provider) {
        const chainId = await safeProviderRequest(provider, 'eth_chainId') as string;
        return Number.parseInt(chainId, 16);
      }
      return 0;
    } catch (error) {
      console.error('[WalletConnection] Failed to get chain ID:', error);
      return 0;
    }
  }, []);

  // Connect to MetaMask using Viem
  const connectWallet = useCallback(async () => {
    const provider = getMetaMaskProvider();
    if (!provider) {
      setWalletState(prev => ({
        ...prev,
        error: 'MetaMask not found. Please install MetaMask extension.'
      }));
      return;
    }

    setWalletState(prev => ({ ...prev, isConnecting: true, error: null }));

    try {
      // CRITICAL: Configure network BEFORE requesting accounts
      // This prevents wallet picker extension errors when the network doesn't exist in MetaMask
      let chainId = await getChainId();

      // If not on StorageHub network, try to switch/add it
      if (chainId !== expectedChainId) {
        const switchSuccess = await switchToStorageHubNetwork();

        if (switchSuccess) {
          // Wait a moment for the switch to complete
          await new Promise(resolve => setTimeout(resolve, 1000));
          chainId = await getChainId();
        } else {
          throw new Error('Failed to switch to StorageHub network. Please switch manually in MetaMask.');
        }
      }

      // NOW request account access (network is configured)
      const accounts = await safeProviderRequest(provider, 'eth_requestAccounts') as string[];

      if (!accounts || accounts.length === 0) {
        throw new Error('No accounts returned from MetaMask');
      }

      const address = accounts[0];

      // Create Viem clients
      if (!provider) {
        throw new Error('MetaMask provider unavailable');
      }
      const transport = custom(provider);

      const publicClientInstance = createPublicClient({
        chain: storageHubChain,
        transport
      });

      const walletClientInstance = createWalletClient({
        chain: storageHubChain,
        transport
      });

      console.log('Viem clients created successfully');

      // Notify parent about ready clients
      if (onClientsReady) {
        onClientsReady({ walletClient: walletClientInstance, publicClient: publicClientInstance });
      }

      // Get balance using Viem
      const balanceWei = await publicClientInstance.getBalance({ address: address as `0x${string}` });
      const balance = formatEther(balanceWei);

      setWalletState({
        address,
        balance,
        chainId,
        isConnecting: false,
        error: null
      });

      // Notify parent about address change
      if (onAddressChange) {
        onAddressChange(address);
      }

      // Check if we're on the correct network
      const isCorrectNetwork = chainId === expectedChainId;
      onWalletConnected(isCorrectNetwork);

      if (!isCorrectNetwork) {
        setWalletState(prev => ({
          ...prev,
          error: `Still on wrong network. Current Chain ID: ${chainId}. Expected: ${expectedChainId}. Please switch manually in MetaMask.`
        }));
      }
    } catch (error) {
      console.error('Failed to connect wallet:', error);
      
      // Provide more specific error messages
      let errorMessage = 'Failed to connect wallet';
      if (error instanceof Error) {
        const errorMsg = error.message;
        if (errorMsg.includes('User rejected') || errorMsg.includes('user rejected')) {
          errorMessage = 'Connection cancelled by user';
        } else if (errorMsg.includes('Unexpected error') || errorMsg.includes('selectExtension')) {
          errorMessage = 'Wallet picker extension error. Try disabling other wallet extensions or refresh the page.';
        } else if (errorMsg.includes('Please switch')) {
          errorMessage = errorMsg;
        } else if (errorMsg.includes('network') || errorMsg.includes('Network')) {
          errorMessage = 'Network configuration error. Please check your MetaMask settings.';
        } else if (errorMsg.includes('JSON-RPC') || errorMsg.includes('RPC')) {
          errorMessage = 'RPC connection error. Please ensure the StorageHub node is running.';
        } else {
          errorMessage = errorMsg;
        }
      }
      
      setWalletState(prev => ({
        ...prev,
        isConnecting: false,
        error: errorMessage
      }));
      onWalletConnected(false);
    }
  }, [onWalletConnected, onClientsReady, onAddressChange, storageHubChain, expectedChainId, switchToStorageHubNetwork, getChainId]);

  // Disconnect wallet
  const disconnectWallet = useCallback(() => {
    setWalletState({
      address: null,
      balance: null,
      chainId: null,
      isConnecting: false,
      error: null
    });
    onWalletConnected(false);

    // Notify parent that clients are no longer available
    if (onClientsReady) {
      onClientsReady(null);
    }

    // Notify parent about address change
    if (onAddressChange) {
      onAddressChange(null);
    }
  }, [onWalletConnected, onClientsReady, onAddressChange]);


  // Copy address to clipboard
  const copyAddress = async () => {
    if (walletState.address) {
      await navigator.clipboard.writeText(walletState.address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // Listen for account/chain changes
  useEffect(() => {
    const provider = getMetaMaskProvider();
    if (provider) {
      const handleAccountsChanged = (...args: unknown[]) => {
        const accounts = args[0] as string[];
        if (accounts.length === 0) {
          disconnectWallet();
        } else if (accounts[0] !== walletState.address) {
          // Reconnect with new account
          void connectWallet();
        }
      };

      const handleChainChanged = (...args: unknown[]) => {
        const chainId = args[0] as string;
        const newChainId = Number.parseInt(chainId, 16);
        setWalletState(prev => ({ ...prev, chainId: newChainId }));

        // Update connection status based on network
        const isCorrectNetwork = newChainId === expectedChainId;
        onWalletConnected(isCorrectNetwork && walletState.address !== null);

        if (!isCorrectNetwork && walletState.address) {
          setWalletState(prev => ({
            ...prev,
            error: `Please switch to StorageHub network (Chain ID: ${expectedChainId}) in MetaMask`
          }));
        } else if (isCorrectNetwork && walletState.address) {
          setWalletState(prev => ({
            ...prev,
            error: null
          }));
        }
      };

      provider.on('accountsChanged', handleAccountsChanged);
      provider.on('chainChanged', handleChainChanged);

      return () => {
        if (provider.removeListener) {
          provider.removeListener('accountsChanged', handleAccountsChanged);
          provider.removeListener('chainChanged', handleChainChanged);
        }
      };
    }
  }, [walletState.address, connectWallet, disconnectWallet, onWalletConnected, expectedChainId]);

  // Check for existing connection on mount
  useEffect(() => {
    const provider = getMetaMaskProvider();
    if (provider && configurationValid) {
      // Check if already connected
      safeProviderRequest(provider, 'eth_accounts')
        .then((result: unknown) => {
          const accounts = result as string[];
          if (accounts.length > 0) {
            void connectWallet();
          }
        })
        .catch((error) => {
          console.error('[WalletConnection] Failed to check existing accounts:', error);
        });
    }
  }, [configurationValid, connectWallet]);

  const formatAddress = (address: string) => {
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  const getChainStatus = () => {
    if (walletState.chainId === expectedChainId) {
      return { status: 'correct', text: 'StorageHub Network', color: 'text-green-600' };
    }
    if (walletState.chainId) {
      return { status: 'wrong', text: `Wrong Network (Chain ID: ${walletState.chainId})`, color: 'text-red-600' };
    }
    return { status: 'unknown', text: 'Unknown Network', color: 'text-gray-600' };
  };

  if (!configurationValid) {
    return (
      <div className="text-center py-12">
        <AlertCircle className="w-12 h-12 text-yellow-600 mx-auto mb-4" />
        <h3 className="text-lg font-semibold text-gray-900 mb-2">Configuration Required</h3>
        <p className="text-gray-600">
          Please complete the SDK configuration before connecting your wallet.
        </p>
      </div>
    );
  }

  const provider = getMetaMaskProvider();
  if (!provider) {
    return (
      <div className="text-center py-12">
        <Wallet className="w-12 h-12 text-gray-400 mx-auto mb-4" />
        <h3 className="text-lg font-semibold text-gray-900 mb-2">MetaMask Required</h3>
        <p className="text-gray-600 mb-6">
          This demo requires MetaMask to interact with the StorageHub blockchain.
        </p>
        <a
          href="https://metamask.io/download/"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-2 px-6 py-3 bg-orange-600 text-white rounded-lg hover:bg-orange-700"
        >
          Install MetaMask
          <ExternalLink className="w-4 h-4" />
        </a>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-4">Wallet Connection</h2>
        <p className="text-gray-600 mb-6">
          Connect your MetaMask wallet to interact with StorageHub SDK features.
        </p>
      </div>

      {!walletConnected ? (
        <div className="max-w-md mx-auto">
          <div className="bg-white border border-gray-200 rounded-lg p-6 text-center">
            <Wallet className="w-12 h-12 text-blue-600 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Connect MetaMask</h3>
            <p className="text-gray-600 mb-6">
              Connect your MetaMask wallet. The app will automatically switch to the StorageHub network if needed.
            </p>

            {walletState.error && (
              <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md">
                <p className="text-sm text-red-600">{walletState.error}</p>
              </div>
            )}

            {/* Debug Info */}
            {walletState.address && (
              <div className="mb-4 p-3 bg-blue-50 border border-blue-200 rounded-md text-left">
                <p className="text-xs text-blue-800 font-semibold mb-2">Debug Info:</p>
                <p className="text-xs text-blue-700">Address: {walletState.address}</p>
                <p className="text-xs text-blue-700">Chain ID: {walletState.chainId}</p>
                <p className="text-xs text-blue-700">Balance: {walletState.balance} SH</p>
                <p className="text-xs text-blue-700">Expected Chain ID: {expectedChainId}</p>
                <p className="text-xs text-blue-700">Network Match: {walletState.chainId === expectedChainId ? '✅ Yes' : '❌ No'}</p>
              </div>
            )}

            <div className="space-y-3">
              <button
                type="button"
                onClick={connectWallet}
                disabled={walletState.isConnecting}
                className="w-full py-3 px-4 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center justify-center gap-2"
              >
                {walletState.isConnecting ? (
                  <>
                    <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                    Connecting...
                  </>
                ) : (
                  <>
                    <Wallet className="w-4 h-4" />
                    Connect & Switch Network
                  </>
                )}
              </button>

              {/* Debug Test Button */}
              <button
                type="button"
                onClick={async () => {
                  try {
                    console.log('=== MetaMask Debug Test ===');
                    const provider = getMetaMaskProvider();
                    console.log('Provider found:', !!provider);
                    console.log('Provider isMetaMask:', provider?.isMetaMask);
                    console.log('window.ethereum type:', Array.isArray(window.ethereum) ? 'array' : typeof window.ethereum);
                    console.log('window.ethereum value:', window.ethereum);

                    if (provider) {
                      try {
                        const chainId = await safeProviderRequest(provider, 'eth_chainId');
                        console.log('Current chainId:', chainId, '(decimal:', Number.parseInt(chainId as string, 16), ')');
                      } catch (e) {
                        console.error('Failed to get chainId:', e);
                      }
                      
                      try {
                        const accounts = await safeProviderRequest(provider, 'eth_accounts');
                        console.log('Current accounts:', accounts);
                      } catch (e) {
                        console.error('Failed to get accounts:', e);
                      }
                    } else {
                      console.warn('No MetaMask provider found');
                      console.log('Available window.ethereum:', window.ethereum);
                    }
                  } catch (e) {
                    console.error('Debug test error:', e);
                    console.error('Error details:', {
                      message: e instanceof Error ? e.message : String(e),
                      stack: e instanceof Error ? e.stack : undefined,
                      name: e instanceof Error ? e.name : undefined
                    });
                  }
                }}
                className="w-full py-2 px-4 bg-gray-500 text-white rounded-lg hover:bg-gray-600 text-sm"
              >
                🔍 Debug MetaMask
              </button>
            </div>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Wallet Info */}
          <div className="bg-green-50 border border-green-200 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 bg-green-100 rounded-full flex items-center justify-center">
                  <Wallet className="w-5 h-5 text-green-600" />
                </div>
                <div>
                  <h3 className="font-semibold text-green-900">Wallet Connected</h3>
                  <p className="text-sm text-green-700">MetaMask successfully connected</p>
                </div>
              </div>
              <button
                type="button"
                onClick={disconnectWallet}
                className="px-3 py-1.5 text-sm bg-green-100 text-green-700 rounded-md hover:bg-green-200"
              >
                Disconnect
              </button>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <div className="block text-sm font-medium text-green-900 mb-1">
                  Address
                </div>
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm text-green-800">
                    {walletState.address ? formatAddress(walletState.address) : ''}
                  </span>
                  <button
                    type="button"
                    onClick={copyAddress}
                    className="p-1 hover:bg-green-200 rounded"
                    title="Copy full address"
                  >
                    {copied ? (
                      <CheckCircle className="w-4 h-4 text-green-600" />
                    ) : (
                      <Copy className="w-4 h-4 text-green-600" />
                    )}
                  </button>
                </div>
              </div>

              <div>
                <div className="block text-sm font-medium text-green-900 mb-1">
                  Balance
                </div>
                <span className="font-mono text-sm text-green-800">
                  {walletState.balance} SH
                </span>
              </div>
            </div>
          </div>

          {/* Network Status */}
          <div className={`p-4 rounded-lg border-2 ${getChainStatus().status === 'correct'
            ? 'border-green-200 bg-green-50'
            : 'border-red-200 bg-red-50'
            }`}>
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-medium text-gray-900">Network Status</h4>
                <p className={`text-sm ${getChainStatus().color}`}>
                  {getChainStatus().text}
                </p>
              </div>

              {getChainStatus().status === 'correct' ? (
                <div className="text-green-600 text-xl">✅</div>
              ) : (
                <div className="text-red-600 text-xl">❌</div>
              )}
            </div>
          </div>

          {/* Network Helper */}
          {getChainStatus().status !== 'correct' && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4">
              <h4 className="font-medium text-red-900 mb-2">Wrong Network</h4>
              <p className="text-sm text-red-800 mb-3">
                You must be connected to the StorageHub network to use this demo. Please switch your MetaMask to the correct network manually.
              </p>
              <div className="text-xs text-red-700 font-mono bg-red-100 p-2 rounded">
                <strong>Required Network:</strong><br />
                Network: StorageHub Solochain EVM<br />
                Chain ID: {expectedChainId}<br />
                RPC: {expectedRpcUrl}<br />
                Symbol: SH
              </div>
            </div>
          )}

          {/* Ready Status */}
          {getChainStatus().status === 'correct' && (
            <div className="bg-green-50 border border-green-200 rounded-lg p-4 text-center">
              <CheckCircle className="w-8 h-8 text-green-600 mx-auto mb-2" />
              <h4 className="font-medium text-green-900 mb-1">Ready to Use StorageHub!</h4>
              <p className="text-sm text-green-700">
                Your wallet is connected to the correct network. You can now use all SDK features.
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
