'use client';

import { useState, useEffect } from 'react';
import { type WalletClient, type PublicClient } from 'viem';
import { Upload, Download, Info, Settings } from 'lucide-react';
import { MspClient } from '@storagehub-sdk/msp-client';
import { StorageHubClient } from '@storagehub-sdk/core';
import { FileManager } from './FileManager';

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

  const connectToMsp = async () => {
    if (!walletClient || !walletAddress) {
      setStatus('❌ Wallet not connected');
      return;
    }

    setMspConnecting(true);
    setStatus('🔄 Connecting to MSP backend...');

    try {
      // Connect to MSP backend (using default from configuration)
      const client = await MspClient.connect({
        baseUrl: 'http://127.0.0.1:8080',
        timeoutMs: 10000
      });

      // Test connection
      const health = await client.getHealth();
      console.log('MSP health:', health);

      // Authenticate with SIWE-style flow
      const chainId = 181222; // StorageHub chain ID
      const { message } = await client.getNonce(walletAddress, chainId);

      setStatus('🔄 Please sign the authentication message...');
      const signature = await walletClient.signMessage({
        account: walletAddress as `0x${string}`,
        message
      });

      const verified = await client.verify(message, signature);
      client.setToken(verified.token);

      // Also create StorageHubClient for blockchain operations
      const storageClient = new StorageHubClient({
        rpcUrl: 'http://127.0.0.1:9888',
        chain: {
          id: 181222,
          name: 'StorageHub Solochain EVM',
          nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
          rpcUrls: { default: { http: ['http://127.0.0.1:9888'] } }
        },
        walletClient
      });

      setMspClient(client);
      setStorageHubClient(storageClient);
      setStatus(`✅ MSP connected and authenticated! User: ${verified.user}`);
    } catch (error) {
      console.error('MSP connection failed:', error);
      setStatus(`❌ MSP connection failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
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
            <button
              onClick={() => setActiveTab('test')}
              className={`flex-1 py-3 px-4 text-center font-medium text-sm ${activeTab === 'test'
                ? 'bg-blue-50 text-blue-700 border-b-2 border-blue-500'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
                }`}
            >
              <Settings className="w-4 h-4 inline mr-2" />
              Connection Test
            </button>
            <button
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
                <button
                  onClick={testViemConnection}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Test Viem Connection
                </button>
                <span className="text-sm text-gray-500">Test blockchain connectivity</span>
              </div>

              {/* MSP Connection */}
              <div className="flex items-center gap-4">
                <button
                  onClick={connectToMsp}
                  disabled={mspConnecting || !!mspClient}
                  className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50"
                >
                  {mspConnecting ? (
                    <>
                      <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin inline mr-2" />
                      Connecting...
                    </>
                  ) : mspClient ? (
                    '✅ MSP Connected'
                  ) : (
                    'Connect to MSP'
                  )}
                </button>
                <span className="text-sm text-gray-500">Connect and authenticate with MSP backend</span>
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
                  <li>• Switch to "File Management" tab to start</li>
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
                <button
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
