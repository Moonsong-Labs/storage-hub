'use client';

import { useState } from 'react';
import type { WalletClient, PublicClient } from 'viem';
import { Upload, Info, Settings } from 'lucide-react';
import { MspClient } from '@storagehub-sdk/msp-client';
import { StorageHubClient, SH_FILE_SYSTEM_PRECOMPILE_ADDRESS } from '@storagehub-sdk/core';
import { FileManager } from './FileManager';
import { loadAppConfig } from '../config/load';
import type { AppConfig } from '../config/types';

interface StorageHubDemoProps {
  walletClient: WalletClient | null;
  publicClient: PublicClient | null;
  walletAddress: string | null;
}

export function StorageHubDemo({ walletClient, publicClient, walletAddress }: StorageHubDemoProps) {
  const [status, setStatus] = useState<string>('');
  const [mspClient, setMspClient] = useState<MspClient | null>(null);
  const [storageHubClient, setStorageHubClient] = useState<StorageHubClient | null>(null);
  const [mspConnecting, setMspConnecting] = useState(false);
  const [activeTab, setActiveTab] = useState<'test' | 'files'>('test');
  const [authMethod, setAuthMethod] = useState<'SIWE' | 'SIWX' | null>(null);

  const testViemConnection = async () => {
    if (!publicClient || !walletClient || !walletAddress) {
      setStatus('❌ Viem clients not ready');
      return;
    }

    try {
      setStatus('🔄 Testing Viem connection...');

      // Test public client
      const blockNumber = await publicClient.getBlockNumber();
      console.log('Current block number:', blockNumber);

      // Test wallet client
      const chainId = await walletClient.getChainId();
      console.log('Chain ID from wallet client:', chainId);

      setStatus(`✅ Viem clients working! Block: ${blockNumber}, Chain: ${chainId}`);
    } catch (error) {
      console.error('Viem test failed:', error);
      setStatus(`❌ Viem test failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const connectToMspSIWE = async () => {
    if (!walletClient || !walletAddress) {
      setStatus('❌ Wallet not connected');
      return;
    }

    setMspConnecting(true);
    setStatus('🔄 Connecting to MSP backend with SIWE...');

    try {
      const appCfg: AppConfig = await loadAppConfig();

      // Connect to MSP backend without sessionProvider (optional now)
      const client = await MspClient.connect({
        baseUrl: appCfg.msp.baseUrl,
        timeoutMs: appCfg.msp.timeoutMs ?? 10000
      });

      // Test connection
      const health = await client.info.getInfo();
      console.log('MSP info:', health);

      // Authenticate with SIWE flow
      setStatus('🔄 Authenticating with SIWE...');
      const domain = appCfg.auth.siweDomain;
      const uri = appCfg.auth.siweUri;
      const session = await client.auth.SIWE(walletClient, domain, uri);

      // Update sessionProvider using new method
      client.setSessionProvider(async () => session);

      const profile = await client.auth.getProfile();

      // Create StorageHubClient for blockchain operations
      const storageClient = new StorageHubClient({
        rpcUrl: appCfg.chain.evmRpcHttpUrl,
        chain: {
          id: appCfg.chain.id,
          name: appCfg.chain.name,
          nativeCurrency: appCfg.chain.nativeCurrency,
          rpcUrls: { default: { http: [appCfg.chain.evmRpcHttpUrl] } }
        },
        walletClient,
        filesystemContractAddress: (appCfg.chain.filesystemPrecompileAddress as `0x${string}` | undefined) ?? SH_FILE_SYSTEM_PRECOMPILE_ADDRESS
      });

      setMspClient(client);
      setStorageHubClient(storageClient);
      setAuthMethod('SIWE');
      setStatus(`✅ MSP connected and authenticated via SIWE! User: ${profile.address}`);
    } catch (error) {
      console.error('MSP SIWE connection failed:', error);
      setStatus(`❌ MSP SIWE connection failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setMspConnecting(false);
    }
  };

  const connectToMspSIWX = async () => {
    if (!walletClient || !walletAddress) {
      setStatus('❌ Wallet not connected');
      return;
    }

    setMspConnecting(true);
    setStatus('🔄 Connecting to MSP backend with SIWX (CAIP-122)...');

    try {
      const appCfg: AppConfig = await loadAppConfig();

      // Connect to MSP backend without sessionProvider (optional now)
      const client = await MspClient.connect({
        baseUrl: appCfg.msp.baseUrl,
        timeoutMs: appCfg.msp.timeoutMs ?? 10000
      });

      // Test connection
      const health = await client.info.getInfo();
      console.log('MSP info:', health);

      // Authenticate with SIWX (CAIP-122) flow - only needs URI
      setStatus('🔄 Authenticating with SIWX (CAIP-122)...');
      const uri = appCfg.auth.siweUri;
      const session = await client.auth.SIWX(walletClient, uri);

      // Update sessionProvider using new method
      client.setSessionProvider(async () => session);

      const profile = await client.auth.getProfile();

      // Create StorageHubClient for blockchain operations
      const storageClient = new StorageHubClient({
        rpcUrl: appCfg.chain.evmRpcHttpUrl,
        chain: {
          id: appCfg.chain.id,
          name: appCfg.chain.name,
          nativeCurrency: appCfg.chain.nativeCurrency,
          rpcUrls: { default: { http: [appCfg.chain.evmRpcHttpUrl] } }
        },
        walletClient,
        filesystemContractAddress: (appCfg.chain.filesystemPrecompileAddress as `0x${string}` | undefined) ?? SH_FILE_SYSTEM_PRECOMPILE_ADDRESS
      });

      setMspClient(client);
      setStorageHubClient(storageClient);
      setAuthMethod('SIWX');
      setStatus(`✅ MSP connected and authenticated via SIWX! User: ${profile.address}`);
    } catch (error) {
      console.error('MSP SIWX connection failed:', error);
      setStatus(`❌ MSP SIWX connection failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setMspConnecting(false);
    }
  };

  if (!walletClient || !publicClient) {
    return (
      <div className="max-w-4xl mx-auto">
        <div className="bg-gray-50 border border-gray-200 rounded-lg p-6 text-center">
          <Info className="w-8 h-8 text-gray-400 mx-auto mb-2" />
          <p className="text-gray-600">Connect your wallet to access StorageHub features</p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
        {/* Tab Navigation */}
        <div className="border-b border-gray-200">
          <nav className="flex">
            <button type="button"
              onClick={() => setActiveTab('test')}
              className={`flex-1 py-3 px-4 text-center font-medium text-sm ${activeTab === 'test'
                ? 'bg-blue-50 text-blue-700 border-b-2 border-blue-500'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                }`}
            >
              <Settings className="w-4 h-4 inline mr-2" />
              Connection Test
            </button>
            <button type="button"
              onClick={() => setActiveTab('files')}
              className={`flex-1 py-3 px-4 text-center font-medium text-sm ${activeTab === 'files'
                ? 'bg-blue-50 text-blue-700 border-b-2 border-blue-500'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                }`}
            >
              <Upload className="w-4 h-4 inline mr-2" />
              File Management
            </button>
          </nav>
        </div>

        {/* Test Tab */}
        {activeTab === 'test' && (
          <div className="p-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">StorageHub SDK Connection Test</h2>
            <p className="text-gray-600 mb-6">
              Test your Viem clients and connect to the MSP backend for file operations.
            </p>

            <div className="space-y-4">
              {/* Viem Test */}
              <div className="flex items-center gap-4">
                <button type="button"
                  onClick={testViemConnection}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Test Viem Connection
                </button>
                <span className="text-sm text-gray-500">Test blockchain connectivity</span>
              </div>

              {/* MSP Connection */}
              <div className="space-y-3">
                <div className="text-sm text-gray-600 mb-2">Choose authentication method:</div>
                <div className="flex flex-wrap gap-3">
                  <button type="button"
                    onClick={connectToMspSIWE}
                    disabled={mspConnecting || !!mspClient}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
                  >
                    {mspConnecting ? (
                      <>
                        <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                        Connecting...
                      </>
                    ) : mspClient ? (
                      '✅ MSP Connected'
                    ) : (
                      <>
                        <span>Connect with SIWE</span>
                        <span className="text-xs opacity-75">(Traditional)</span>
                      </>
                    )}
                  </button>
                  <button type="button"
                    onClick={connectToMspSIWX}
                    disabled={mspConnecting || !!mspClient}
                    className="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
                  >
                    {mspConnecting ? (
                      <>
                        <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                        Connecting...
                      </>
                    ) : mspClient ? (
                      '✅ MSP Connected'
                    ) : (
                      <>
                        <span>Connect with SIWX</span>
                        <span className="text-xs opacity-75">(CAIP-122)</span>
                      </>
                    )}
                  </button>
                </div>
                {mspClient && authMethod && (
                  <div className="text-xs text-gray-500 mt-1">
                    Connected via {authMethod}
                  </div>
                )}
              </div>
            </div>

            {status && (
              <div className="mt-6 p-3 bg-blue-50 border border-blue-200 rounded-md">
                <p className="text-sm text-blue-800">{status}</p>
              </div>
            )}

            {mspClient && (
              <div className="mt-6 bg-green-50 border border-green-200 rounded-lg p-4">
                <h4 className="font-semibold text-green-800 mb-2">✅ Ready for File Operations!</h4>
                <ul className="text-sm text-green-700 space-y-1">
                  <li>• Viem clients connected to StorageHub blockchain</li>
                  <li>• MSP backend authenticated and ready</li>
                  <li>• File upload/download functionality available</li>
                  <li>• Switch to &quot;File Management&quot; tab to start</li>
                </ul>
              </div>
            )}
          </div>
        )}

        {/* Files Tab */}
        {activeTab === 'files' && (
          <div className="p-6">
            {mspClient && storageHubClient ? (
              <FileManager
                walletClient={walletClient}
                publicClient={publicClient}
                walletAddress={walletAddress}
                mspClient={mspClient}
                storageHubClient={storageHubClient}
              />
            ) : (
              <div className="text-center py-12">
                <Info className="w-12 h-12 text-gray-400 mx-auto mb-4" />
                <h3 className="text-lg font-semibold text-gray-900 mb-2">MSP Connection Required</h3>
                <p className="text-gray-600 mb-4">
                  Please connect to the MSP backend first to access file management features.
                </p>
                <button type="button"
                  onClick={() => setActiveTab('test')}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Go to Connection Test
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
