'use client';

import { useState, useCallback, useMemo, useEffect } from 'react';
import { Settings, Wallet, Database, CheckCircle, AlertCircle, ExternalLink } from 'lucide-react';
import { createWalletClient, createPublicClient, custom, formatEther, getAddress, type WalletClient, type PublicClient } from 'viem';
import { StorageHubClient } from '@storagehub-sdk/core';
import { MspClient } from '@storagehub-sdk/msp-client';
import { FileManager } from './FileManager';
import { loadAppConfig } from '../config/load';
import type { AppConfig } from '../config/types';

export function OnePageDemo() {
  const [config, setConfig] = useState({
    rpcUrl: 'http://127.0.0.1:9888',
    chainId: 181222,
    mspUrl: 'http://127.0.0.1:8080',
    fsAddress: undefined as `0x${string}` | undefined
  });

  // Load runtime configuration
  useEffect(() => {
    const run = async () => {
      try {
        const appCfg: AppConfig = await loadAppConfig();
        setConfig({
          rpcUrl: appCfg.chain.evmRpcHttpUrl,
          chainId: appCfg.chain.id,
          mspUrl: appCfg.msp.baseUrl,
          fsAddress: appCfg.chain.filesystemPrecompileAddress
        });
      } catch (e) {
        // Keep defaults on failure
        console.warn('Failed to load app config, using defaults', e);
      }
    };
    run();
  }, []);

  // Wallet state
  const [walletClient, setWalletClient] = useState<WalletClient | null>(null);
  const [publicClient, setPublicClient] = useState<PublicClient | null>(null);
  const [walletAddress, setWalletAddress] = useState<string | null>(null);
  const [walletBalance, setWalletBalance] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [walletError, setWalletError] = useState<string | null>(null);

  // MSP state
  const [mspClient, setMspClient] = useState<MspClient | null>(null);
  const [storageHubClient, setStorageHubClient] = useState<StorageHubClient | null>(null);
  const [isMspConnecting, setIsMspConnecting] = useState(false);
  const [mspError, setMspError] = useState<string | null>(null);

  // Define the StorageHub chain configuration
  const storageHubChain = useMemo(() => ({
    id: config.chainId,
    name: 'StorageHub Solochain EVM',
    nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
    rpcUrls: { default: { http: [config.rpcUrl] } },
  }), [config.chainId, config.rpcUrl]);

  // Check if MetaMask is available (client-side only to avoid hydration mismatch)
  const [isMetaMaskAvailable, setIsMetaMaskAvailable] = useState<boolean | null>(null);

  // Check MetaMask availability after component mounts (client-side only)
  useEffect(() => {
    setIsMetaMaskAvailable(typeof window !== 'undefined' && typeof window.ethereum !== 'undefined');
  }, []);

  // Connect wallet function
  const connectWallet = useCallback(async () => {
    if (isMetaMaskAvailable === false) {
      setWalletError('MetaMask is not installed. Please install MetaMask to continue.');
      return;
    }

    if (isMetaMaskAvailable === null) {
      // Still checking, shouldn't happen but safety check
      return;
    }

    setIsConnecting(true);
    setWalletError(null);

    try {
      console.log('🔄 Step 1: Requesting account access...');
      // Request account access
      const accounts = await window.ethereum!.request({ method: 'eth_requestAccounts' }) as string[];

      if (!accounts || accounts.length === 0) {
        throw new Error('No accounts returned from MetaMask');
      }

      const rawAddress = accounts[0] as `0x${string}`;
      const address = getAddress(rawAddress); // Ensure proper checksum format
      console.log('✅ Step 1 Complete: Got address (raw):', rawAddress);
      console.log('✅ Step 1 Complete: Got address (checksum):', address);

      console.log('🔄 Step 2: Checking current network...');
      // Check current network directly via MetaMask
      const currentChainIdHex = await window.ethereum!.request({ method: 'eth_chainId' }) as string;
      const currentChainId = parseInt(currentChainIdHex, 16);
      console.log('Current Chain ID:', currentChainId, 'Expected:', config.chainId);

      // Switch to StorageHub chain if needed
      if (currentChainId !== config.chainId) {
        console.log('🔄 Step 3: Switching to StorageHub network...');
        try {
          await window.ethereum!.request({
            method: 'wallet_switchEthereumChain',
            params: [{ chainId: `0x${config.chainId.toString(16)}` }],
          });
          console.log('✅ Network switched successfully');
        } catch (switchError: unknown) {
          // If chain doesn't exist, add it
          if (switchError && typeof switchError === 'object' && 'code' in switchError && (switchError as { code: number }).code === 4902) {
            console.log('🔄 Network not found, adding StorageHub network...');
            await window.ethereum!.request({
              method: 'wallet_addEthereumChain',
              params: [{
                chainId: `0x${config.chainId.toString(16)}`,
                chainName: storageHubChain.name,
                nativeCurrency: storageHubChain.nativeCurrency,
                rpcUrls: [config.rpcUrl],
              }],
            });
            console.log('✅ Network added successfully');
          } else {
            console.error('Network switch failed:', switchError);
            const errMsg =
              switchError && typeof switchError === 'object' && 'message' in switchError
                ? (switchError as { message: string }).message
                : String(switchError);
            throw new Error(`Failed to switch network: ${errMsg}`);
          }
        }
      }

      console.log('🔄 Step 4: Creating Viem clients...');
      // Create Viem clients with error handling
      const transport = custom(window.ethereum!, {
        // Add retries and timeout for better reliability
        retryCount: 3,
        retryDelay: 1000,
      });

      const publicClient = createPublicClient({
        chain: storageHubChain,
        transport,
      });

      const viemWalletClient = createWalletClient({
        chain: storageHubChain,
        transport,
        account: address,
      });
      const walletClient = (viemWalletClient as unknown) as WalletClient;

      console.log('✅ Step 4 Complete: Viem clients created');

      console.log('🔄 Step 5: Getting wallet balance...');
      // Get balance with error handling
      let formattedBalance = '0';
      try {
        const balance = await publicClient.getBalance({ address });
        formattedBalance = formatEther(balance);
        console.log('✅ Step 5 Complete: Balance retrieved:', formattedBalance, 'SH');
      } catch (balanceError) {
        console.warn('⚠️ Could not fetch balance, using default:', balanceError);
        // Don't fail the connection just because balance fetch failed
      }

      // Set state
      setWalletClient(walletClient);
      setPublicClient(publicClient);
      setWalletAddress(address);
      setWalletBalance(formattedBalance);

      console.log('✅ Wallet connection complete:', {
        address,
        chainId: config.chainId,
        balance: formattedBalance
      });

    } catch (error: unknown) {
      console.error('❌ Wallet connection failed:', error);

      // Provide more specific error messages
      let errorMessage = 'Failed to connect wallet';
      const errorMsg = error && typeof error === 'object' && 'message' in error ? (error as { message: string }).message : '';
      if (errorMsg.includes('User rejected')) {
        errorMessage = 'Connection cancelled by user';
      } else if (errorMsg.includes('network')) {
        errorMessage = 'Network configuration error. Please check your MetaMask settings.';
      } else if (errorMsg.includes('JSON-RPC')) {
        errorMessage = 'RPC connection error. Please ensure the StorageHub node is running.';
      } else if (errorMsg) {
        errorMessage = errorMsg;
      }

      setWalletError(errorMessage);
    } finally {
      setIsConnecting(false);
    }
  }, [isMetaMaskAvailable, config.chainId, config.rpcUrl, storageHubChain]);

  // Connect MSP function
  const connectMsp = useCallback(async () => {
    if (!walletClient || !walletAddress) return;

    setIsMspConnecting(true);
    setMspError(null);

    try {
      // Create MSP client
      const mspClient = await MspClient.connect({ baseUrl: config.mspUrl });

      // REAL SIWE AUTHENTICATION ONLY
      console.log('🔐 MSP Authentication: Starting SIWE flow...');
      console.log('- Address:', walletAddress);
      console.log('- Chain ID:', config.chainId);

      await mspClient.auth.SIWE(walletClient as WalletClient);
      console.log('✅ Authentication completed successfully');

      // Get user profile to verify authentication
      const profile = await mspClient.auth.getProfile();
      console.log('Authenticated user:', profile);

      // Token management is handled internally by the SDK
      console.log('✅ MSP client authenticated successfully');

      // Create StorageHub client
      const storageHubClient = new StorageHubClient({
        rpcUrl: config.rpcUrl,
        chain: storageHubChain,
        walletClient: (walletClient as unknown) as WalletClient,
        filesystemContractAddress: config.fsAddress ? (config.fsAddress as `0x${string}`) : undefined
      });

      setMspClient(mspClient);
      setStorageHubClient(storageHubClient);

      console.log('✅ MSP connected and authenticated');

    } catch (error) {
      console.error('MSP connection failed:', error);
      setMspError(error instanceof Error ? error.message : 'Failed to connect to MSP');
    } finally {
      setIsMspConnecting(false);
    }
  }, [walletClient, walletAddress, config, storageHubChain]);

  // Listen for account changes
  useEffect(() => {
    if (isMetaMaskAvailable !== true) return;

    const handleAccountsChanged = (accounts: unknown) => {
      const accountList = accounts as string[];
      if (accountList.length === 0) {
        // User disconnected
        setWalletClient(null);
        setPublicClient(null);
        setWalletAddress(null);
        setWalletBalance(null);
        setMspClient(null);
        setStorageHubClient(null);
      } else if (accountList[0] !== walletAddress) {
        // Account changed, reconnect
        connectWallet();
      }
    };

    const handleChainChanged = () => {
      // Chain changed, reconnect
      connectWallet();
    };

    if (window.ethereum) {
      window.ethereum.on('accountsChanged', handleAccountsChanged);
      window.ethereum.on('chainChanged', handleChainChanged);
    }

    return () => {
      if (window.ethereum) {
        window.ethereum.removeListener!('accountsChanged', handleAccountsChanged);
        window.ethereum.removeListener!('chainChanged', handleChainChanged);
      }
    };
  }, [isMetaMaskAvailable, walletAddress, connectWallet]);

  return (
    <div className="min-h-screen bg-black text-gray-100">
      <div className="max-w-4xl mx-auto p-8">
        {/* Header */}
        <header className="mb-8">
          <h1 className="text-3xl font-bold text-blue-400 mb-2">StorageHub SDK Demo</h1>
          <p className="text-gray-400">One-page demo with dark theme</p>
        </header>

        {/* Configuration Section */}
        <section className="mb-8 p-6 bg-gray-900 rounded-lg border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Settings className="h-5 w-5 text-blue-400" />
            <h2 className="text-xl font-semibold">Configuration</h2>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">RPC URL</label>
              <input
                type="text"
                value={config.rpcUrl}
                onChange={(e) => setConfig(prev => ({ ...prev, rpcUrl: e.target.value }))}
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-gray-100 focus:border-blue-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">Chain ID</label>
              <input
                type="number"
                value={config.chainId}
                onChange={(e) => setConfig(prev => ({ ...prev, chainId: parseInt(e.target.value) }))}
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-gray-100 focus:border-blue-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">MSP URL</label>
              <input
                type="text"
                value={config.mspUrl}
                onChange={(e) => setConfig(prev => ({ ...prev, mspUrl: e.target.value }))}
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-gray-100 focus:border-blue-500 focus:outline-none"
              />
            </div>
          </div>

          {/* (Mock authentication removed) */}
        </section>

        {/* Wallet Section */}
        <section className="mb-8 p-6 bg-gray-900 rounded-lg border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Wallet className="h-5 w-5 text-blue-400" />
            <h2 className="text-xl font-semibold">Wallet Connection</h2>
            {walletAddress && (
              <div className="flex items-center gap-1 ml-auto">
                <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                <span className="text-xs text-green-400">Connected</span>
              </div>
            )}
          </div>

          {!walletAddress ? (
            <div className="space-y-4">
              {/* Show loading state during MetaMask detection */}
              {isMetaMaskAvailable === null && (
                <div className="text-center py-8">
                  <p className="text-gray-400 mb-4">Checking for MetaMask...</p>
                </div>
              )}

              {/* Show MetaMask not available warning */}
              {isMetaMaskAvailable === false && (
                <div className="flex items-center gap-2 p-3 bg-yellow-900/20 border border-yellow-900/50 rounded-md text-yellow-400">
                  <AlertCircle className="h-4 w-4" />
                  <span className="text-sm">MetaMask not detected.</span>
                  <a
                    href="https://metamask.io/download/"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-400 hover:text-blue-300 inline-flex items-center gap-1"
                  >
                    Install MetaMask <ExternalLink className="h-3 w-3" />
                  </a>
                </div>
              )}

              {/* Show connect button when MetaMask is available */}
              {isMetaMaskAvailable === true && (
                <div className="text-center py-8">
                  <p className="text-gray-400 mb-4">Connect your MetaMask wallet to continue</p>
                  <button
                    onClick={connectWallet}
                    disabled={isConnecting}
                    className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors"
                  >
                    {isConnecting ? 'Connecting...' : 'Connect Wallet'}
                  </button>
                </div>
              )}

              {walletError && (
                <div className="flex items-center gap-2 p-3 bg-red-900/20 border border-red-900/50 rounded-md text-red-400">
                  <AlertCircle className="h-4 w-4" />
                  <span className="text-sm">{walletError}</span>
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="p-3 bg-gray-800 rounded-md">
                  <div className="text-xs text-gray-400 mb-1">Address</div>
                  <div className="text-sm font-mono">{walletAddress.slice(0, 6)}...{walletAddress.slice(-4)}</div>
                </div>
                <div className="p-3 bg-gray-800 rounded-md">
                  <div className="text-xs text-gray-400 mb-1">Balance</div>
                  <div className="text-sm">{walletBalance ? `${parseFloat(walletBalance).toFixed(4)} SH` : 'Loading...'}</div>
                </div>
              </div>

              <div className="p-3 bg-gray-800 rounded-md">
                <div className="text-xs text-gray-400 mb-1">Network</div>
                <div className="text-sm">StorageHub Solochain EVM (Chain ID: {config.chainId})</div>
              </div>
            </div>
          )}
        </section>

        {/* MSP Connection Section */}
        {walletAddress && (
          <section className="mb-8 p-6 bg-gray-900 rounded-lg border border-gray-800">
            <div className="flex items-center gap-2 mb-4">
              <Database className="h-5 w-5 text-blue-400" />
              <h2 className="text-xl font-semibold">MSP Connection</h2>
              {mspClient && (
                <div className="flex items-center gap-1 ml-auto">
                  <div className="w-2 h-2 bg-green-400 rounded-full"></div>
                  <span className="text-xs text-green-400">Connected</span>
                </div>
              )}
            </div>

            {!mspClient ? (
              <div className="space-y-4">
                <div className="text-center py-8">
                  <p className="text-gray-400 mb-4">Connect to MSP backend to access storage features</p>
                  <button
                    onClick={connectMsp}
                    disabled={isMspConnecting}
                    className="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors"
                  >
                    {isMspConnecting ? 'Connecting...' : 'Connect to MSP'}
                  </button>
                </div>

                {mspError && (
                  <div className="flex items-center gap-2 p-3 bg-red-900/20 border border-red-900/50 rounded-md text-red-400">
                    <AlertCircle className="h-4 w-4" />
                    <span className="text-sm">{mspError}</span>
                  </div>
                )}
              </div>
            ) : (
              <div className="space-y-3">
                <div className="flex items-center gap-2 p-3 bg-green-900/20 border border-green-900/50 rounded-md text-green-400">
                  <CheckCircle className="h-4 w-4" />
                  <span className="text-sm">MSP connected and authenticated</span>
                </div>
              </div>
            )}
          </section>
        )}

        {/* Storage Actions Section */}
        <section className="p-6 bg-gray-900 rounded-lg border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Database className="h-5 w-5 text-blue-400" />
            <h2 className="text-xl font-semibold">Storage Actions</h2>
          </div>

          {mspClient && storageHubClient && walletClient && publicClient && walletAddress ? (
            <FileManager
              walletClient={walletClient}
              publicClient={publicClient}
              walletAddress={walletAddress}
              mspClient={mspClient}
              storageHubClient={storageHubClient}
            />
          ) : (
            <div className="text-center py-12 text-gray-500">
              <Database className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>Connect your wallet and MSP to access storage features</p>
              <div className="mt-4 text-sm">
                <div className="flex items-center justify-center gap-4">
                  <span className={`flex items-center gap-1 ${walletAddress ? 'text-green-400' : 'text-gray-500'}`}>
                    {walletAddress ? <CheckCircle className="h-3 w-3" /> : <div className="w-3 h-3 border border-gray-500 rounded-full" />}
                    Wallet
                  </span>
                  <span className={`flex items-center gap-1 ${mspClient ? 'text-green-400' : 'text-gray-500'}`}>
                    {mspClient ? <CheckCircle className="h-3 w-3" /> : <div className="w-3 h-3 border border-gray-500 rounded-full" />}
                    MSP
                  </span>
                </div>
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}