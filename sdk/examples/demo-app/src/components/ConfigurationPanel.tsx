'use client';

import { useState, useEffect } from 'react';
import { CheckCircle, AlertCircle, Settings, Globe, Database } from 'lucide-react';

interface ConfigurationPanelProps {
  onConfigurationValid: (valid: boolean) => void;
  configurationValid: boolean;
  environmentReady: boolean;
}

interface MspConfig {
  baseUrl: string;
  timeout: number;
  customHeaders: Record<string, string>;
}

interface BlockchainConfig {
  rpcUrl: string;
  chainId: number;
  chainName: string;
  nativeCurrency: {
    name: string;
    symbol: string;
    decimals: number;
  };
}

export function ConfigurationPanel({
  onConfigurationValid,
  configurationValid,
  environmentReady
}: ConfigurationPanelProps) {
  const [mspConfig, setMspConfig] = useState<MspConfig>({
    baseUrl: 'http://127.0.0.1:8080',
    timeout: 30000,
    customHeaders: {}
  });

  const [blockchainConfig, setBlockchainConfig] = useState<BlockchainConfig>({
    rpcUrl: 'ws://127.0.0.1:9888',
    chainId: 181222,
    chainName: 'StorageHub Solochain EVM',
    nativeCurrency: {
      name: 'StorageHub',
      symbol: 'SH',
      decimals: 18
    }
  });

  const [mspStatus, setMspStatus] = useState<'checking' | 'connected' | 'error'>('checking');
  const [blockchainStatus, setBlockchainStatus] = useState<'checking' | 'connected' | 'error'>('checking');
  const [customHeaders, setCustomHeaders] = useState('');

  // Test MSP connection
  const testMspConnection = async () => {
    if (!environmentReady) {
      setMspStatus('error');
      return false;
    }

    setMspStatus('checking');
    try {
      const response = await fetch(`${mspConfig.baseUrl}/health`, {
        method: 'GET',
        timeout: mspConfig.timeout,
        headers: mspConfig.customHeaders,
      } as RequestInit);

      if (response.ok) {
        const health = await response.json();
        setMspStatus('connected');
        return true;
      } else {
        setMspStatus('error');
        return false;
      }
    } catch (error) {
      console.error('MSP connection test failed:', error);
      setMspStatus('error');
      return false;
    }
  };

  // Test blockchain connection
  const testBlockchainConnection = async () => {
    if (!environmentReady) {
      setBlockchainStatus('error');
      return false;
    }

    setBlockchainStatus('checking');
    try {
      await new Promise<void>((resolve, reject) => {
        const ws = new WebSocket(blockchainConfig.rpcUrl);
        const timeout = setTimeout(() => {
          ws.close();
          reject(new Error('Connection timeout'));
        }, 5000);

        ws.onopen = () => {
          clearTimeout(timeout);
          // Send a basic RPC request to test functionality
          ws.send(JSON.stringify({
            id: 1,
            jsonrpc: '2.0',
            method: 'system_health',
            params: []
          }));
        };

        ws.onmessage = (event) => {
          try {
            const response = JSON.parse(event.data);
            if (response.id === 1) {
              ws.close();
              resolve();
            }
          } catch (e) {
            ws.close();
            reject(e);
          }
        };

        ws.onerror = () => {
          clearTimeout(timeout);
          reject(new Error('WebSocket connection failed'));
        };
      });

      setBlockchainStatus('connected');
      return true;
    } catch (error) {
      console.error('Blockchain connection test failed:', error);
      setBlockchainStatus('error');
      return false;
    }
  };

  // Test all connections
  const testConnections = async () => {
    const [mspOk, blockchainOk] = await Promise.all([
      testMspConnection(),
      testBlockchainConnection()
    ]);

    const allValid = mspOk && blockchainOk;
    onConfigurationValid(allValid);
  };

  // Parse custom headers
  const parseCustomHeaders = (headersString: string): Record<string, string> => {
    try {
      if (!headersString.trim()) return {};
      return JSON.parse(headersString);
    } catch {
      return {};
    }
  };

  // Handle custom headers change
  const handleCustomHeadersChange = (value: string) => {
    setCustomHeaders(value);
    const parsed = parseCustomHeaders(value);
    setMspConfig(prev => ({ ...prev, customHeaders: parsed }));
  };

  // Auto-test connections when config changes
  useEffect(() => {
    if (environmentReady) {
      const timeoutId = setTimeout(testConnections, 1000);
      return () => clearTimeout(timeoutId);
    }
  }, [mspConfig, blockchainConfig, environmentReady]);

  const getStatusIcon = (status: 'checking' | 'connected' | 'error') => {
    switch (status) {
    case 'connected': return <CheckCircle className="w-5 h-5 text-green-600" />;
    case 'error': return <AlertCircle className="w-5 h-5 text-red-600" />;
    case 'checking': return <div className="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />;
    }
  };

  const getStatusColor = (status: 'checking' | 'connected' | 'error') => {
    switch (status) {
    case 'connected': return 'border-green-200 bg-green-50';
    case 'error': return 'border-red-200 bg-red-50';
    case 'checking': return 'border-blue-200 bg-blue-50';
    }
  };

  if (!environmentReady) {
    return (
      <div className="text-center py-12">
        <AlertCircle className="w-12 h-12 text-yellow-600 mx-auto mb-4" />
        <h3 className="text-lg font-semibold text-gray-900 mb-2">Environment Required</h3>
        <p className="text-gray-600">
          Please set up the environment first before configuring SDK connections.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-4">SDK Configuration</h2>
        <p className="text-gray-600 mb-6">
          Configure connections to the MSP backend and StorageHub blockchain node.
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* MSP Backend Configuration */}
        <div className={`p-6 rounded-lg border-2 ${getStatusColor(mspStatus)}`}>
          <div className="flex items-center gap-3 mb-4">
            <Database className="w-6 h-6 text-blue-600" />
            <h3 className="text-lg font-semibold text-gray-900">MSP Backend</h3>
            {getStatusIcon(mspStatus)}
          </div>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Base URL
              </label>
              <input
                type="url"
                value={mspConfig.baseUrl}
                onChange={(e) => setMspConfig(prev => ({ ...prev, baseUrl: e.target.value }))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="http://127.0.0.1:8080"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Timeout (ms)
              </label>
              <input
                type="number"
                value={mspConfig.timeout}
                onChange={(e) => setMspConfig(prev => ({ ...prev, timeout: parseInt(e.target.value) || 30000 }))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                min="1000"
                max="60000"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Custom Headers (JSON)
              </label>
              <textarea
                value={customHeaders}
                onChange={(e) => handleCustomHeadersChange(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                rows={3}
                placeholder='{"Authorization": "Bearer token"}'
              />
            </div>

            <button
              onClick={testMspConnection}
              disabled={mspStatus === 'checking'}
              className="w-full py-2 px-4 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              {mspStatus === 'checking' ? 'Testing...' : 'Test Connection'}
            </button>
          </div>
        </div>

        {/* Blockchain Configuration */}
        <div className={`p-6 rounded-lg border-2 ${getStatusColor(blockchainStatus)}`}>
          <div className="flex items-center gap-3 mb-4">
            <Globe className="w-6 h-6 text-purple-600" />
            <h3 className="text-lg font-semibold text-gray-900">Blockchain Node</h3>
            {getStatusIcon(blockchainStatus)}
          </div>

          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                RPC URL
              </label>
              <input
                type="url"
                value={blockchainConfig.rpcUrl}
                onChange={(e) => setBlockchainConfig(prev => ({ ...prev, rpcUrl: e.target.value }))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                placeholder="ws://127.0.0.1:9888"
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Chain ID
                </label>
                <input
                  type="number"
                  value={blockchainConfig.chainId}
                  onChange={(e) => setBlockchainConfig(prev => ({ ...prev, chainId: parseInt(e.target.value) || 181222 }))}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Symbol
                </label>
                <input
                  type="text"
                  value={blockchainConfig.nativeCurrency.symbol}
                  onChange={(e) => setBlockchainConfig(prev => ({
                    ...prev,
                    nativeCurrency: { ...prev.nativeCurrency, symbol: e.target.value }
                  }))}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Chain Name
              </label>
              <input
                type="text"
                value={blockchainConfig.chainName}
                onChange={(e) => setBlockchainConfig(prev => ({ ...prev, chainName: e.target.value }))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <button
              onClick={testBlockchainConnection}
              disabled={blockchainStatus === 'checking'}
              className="w-full py-2 px-4 bg-purple-600 text-white rounded-md hover:bg-purple-700 disabled:opacity-50"
            >
              {blockchainStatus === 'checking' ? 'Testing...' : 'Test Connection'}
            </button>
          </div>
        </div>
      </div>

      {/* Configuration Status */}
      <div className={`p-4 rounded-lg border-2 ${configurationValid ? 'border-green-200 bg-green-50' : 'border-yellow-200 bg-yellow-50'}`}>
        <div className="flex items-center gap-3">
          {configurationValid ? (
            <CheckCircle className="w-6 h-6 text-green-600" />
          ) : (
            <Settings className="w-6 h-6 text-yellow-600" />
          )}
          <div>
            <h4 className="font-semibold text-gray-900">
              {configurationValid ? 'Configuration Valid' : 'Configuration Pending'}
            </h4>
            <p className="text-sm text-gray-600">
              {configurationValid
                ? 'All services are connected and ready for use.'
                : 'Complete the configuration and test connections to proceed.'
              }
            </p>
          </div>
        </div>
      </div>

      {/* MetaMask Network Helper */}
      {configurationValid && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
          <h4 className="font-medium text-blue-900 mb-3">MetaMask Network Configuration</h4>
          <p className="text-sm text-blue-800 mb-3">
            Add this network to MetaMask to interact with the local StorageHub environment:
          </p>
          <div className="bg-white border border-blue-200 rounded p-3 text-sm font-mono">
            <div className="grid grid-cols-2 gap-2">
              <span className="text-gray-600">Network Name:</span>
              <span>{blockchainConfig.chainName}</span>
              <span className="text-gray-600">RPC URL:</span>
              <span>{blockchainConfig.rpcUrl.replace('ws://', 'http://')}</span>
              <span className="text-gray-600">Chain ID:</span>
              <span>{blockchainConfig.chainId}</span>
              <span className="text-gray-600">Currency Symbol:</span>
              <span>{blockchainConfig.nativeCurrency.symbol}</span>
            </div>
          </div>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded text-sm">
            <p className="text-yellow-800">
              <strong>Note:</strong> The environment runs multiple nodes on different ports:
            </p>
            <ul className="text-yellow-700 text-xs mt-1 font-mono">
              <li>• User node: ws://127.0.0.1:9888 (main RPC endpoint)</li>
              <li>• BSP node: ws://127.0.0.1:9666</li>
              <li>• MSP nodes: ws://127.0.0.1:9777, ws://127.0.0.1:9778</li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}
