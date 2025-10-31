'use client';

import { useState, useEffect, useCallback } from 'react';
import { Play, Square, RefreshCcw } from 'lucide-react';

interface EnvironmentSetupProps {
  onEnvironmentReady: (ready: boolean) => void;
  environmentReady: boolean;
}

interface ServiceStatus {
  name: string;
  url: string;
  status: 'checking' | 'running' | 'stopped' | 'error';
  description: string;
}

export function EnvironmentSetup({ onEnvironmentReady, environmentReady }: EnvironmentSetupProps) {
  const [services, setServices] = useState<ServiceStatus[]>([
    {
      name: 'StorageHub Node',
      url: 'ws://127.0.0.1:9888',
      status: 'checking',
      description: 'Blockchain node with EVM support'
    },
    {
      name: 'MSP Backend',
      url: 'http://127.0.0.1:8080',
      status: 'checking',
      description: 'Main Storage Provider REST API'
    },
    {
      name: 'PostgreSQL',
      url: 'internal',
      status: 'checking',
      description: 'Indexer database service'
    }
  ]);

  const [isChecking, setIsChecking] = useState(false);

  const checkServiceStatus = useCallback(async (service: ServiceStatus): Promise<ServiceStatus> => {
    if (service.url === 'internal') {
      // For PostgreSQL, we check if MSP backend can connect to it via health endpoint
      try {
        const response = await fetch('http://127.0.0.1:8080/health', {
          method: 'GET',
          timeout: 5000,
        } as RequestInit);

        if (response.ok) {
          const health = await response.json();
          const pgStatus = health.components?.postgres?.status;
          return {
            ...service,
            status: pgStatus === 'healthy' ? 'running' : 'error'
          };
        }
        return { ...service, status: 'stopped' };
      } catch {
        return { ...service, status: 'stopped' };
      }
    }

    if (service.url.startsWith('ws://')) {
      // Check WebSocket connection
      try {
        await new Promise<void>((resolve, reject) => {
          const ws = new WebSocket(service.url);
          const timeout = setTimeout(() => {
            ws.close();
            reject(new Error('Timeout'));
          }, 5000);

          ws.onopen = () => {
            clearTimeout(timeout);
            ws.close();
            resolve();
          };

          ws.onerror = () => {
            clearTimeout(timeout);
            reject(new Error('Connection failed'));
          };
        });
        return { ...service, status: 'running' };
      } catch {
        return { ...service, status: 'stopped' };
      }
    }

    if (service.url.startsWith('http://')) {
      // Check HTTP endpoint
      try {
        const response = await fetch(`${service.url}/health`, {
          method: 'GET',
          timeout: 5000,
        } as RequestInit);

        return {
          ...service,
          status: response.ok ? 'running' : 'error'
        };
      } catch {
        return { ...service, status: 'stopped' };
      }
    }

    return { ...service, status: 'error' };
  }, []);

  const checkAllServices = useCallback(async () => {
    setIsChecking(true);

    const updatedServices = await Promise.all(
      services.map(service => checkServiceStatus(service))
    );

    setServices(updatedServices);

    const allRunning = updatedServices.every(service => service.status === 'running');
    onEnvironmentReady(allRunning);

    setIsChecking(false);
  }, [services, onEnvironmentReady, checkServiceStatus]);

  useEffect(() => {
    void checkAllServices();
    // Check services every 30 seconds
    const interval = setInterval(checkAllServices, 30000);
    return () => clearInterval(interval);
  }, [checkAllServices]);

  const getStatusColor = (status: ServiceStatus['status']) => {
    switch (status) {
    case 'running': return 'text-green-600 bg-green-50 border-green-200';
    case 'stopped': return 'text-red-600 bg-red-50 border-red-200';
    case 'error': return 'text-red-600 bg-red-50 border-red-200';
    case 'checking': return 'text-yellow-600 bg-yellow-50 border-yellow-200';
    default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const getStatusIcon = (status: ServiceStatus['status']) => {
    switch (status) {
    case 'running': return '🟢';
    case 'stopped': return '🔴';
    case 'error': return '❌';
    case 'checking': return '🟡';
    default: return '⚪';
    }
  };

  return (
    <div className="space-y-6">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-4">Environment Setup</h2>
        <p className="text-gray-600 mb-6">
          The demo requires a local StorageHub environment with blockchain node, MSP backend, and database services.
        </p>
      </div>

      {/* Service Status */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-gray-900">Service Status</h3>
          <button type="button"
            onClick={checkAllServices}
            disabled={isChecking}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-blue-50 text-blue-600 rounded-md hover:bg-blue-100 disabled:opacity-50"
          >
            <RefreshCcw className={`w-4 h-4 ${isChecking ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>

        <div className="grid gap-3">
          {services.map((service) => (
            <div
              key={service.name}
              className={`p-4 rounded-lg border ${getStatusColor(service.status)}`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <span className="text-lg">{getStatusIcon(service.status)}</span>
                  <div>
                    <h4 className="font-medium">{service.name}</h4>
                    <p className="text-sm opacity-80">{service.description}</p>
                    {service.url !== 'internal' && (
                      <p className="text-xs opacity-60 font-mono">{service.url}</p>
                    )}
                  </div>
                </div>
                <span className="text-sm font-medium capitalize">
                  {service.status}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Setup Instructions */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-6">
        <h3 className="text-lg font-semibold text-blue-900 mb-4">Setup Instructions</h3>

        {environmentReady ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2 text-green-600">
              <span>✅</span>
              <span className="font-medium">Environment is ready!</span>
            </div>
            <p className="text-blue-800">
              All services are running. You can proceed to configure the SDK connections.
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="space-y-2">
              <h4 className="font-medium text-blue-900">1. Build Docker Images</h4>
              <div className="bg-gray-900 text-green-400 p-3 rounded font-mono text-sm">
                pnpm demo:build
              </div>
              <p className="text-sm text-blue-800">
                This builds the StorageHub node and MSP backend Docker images.
              </p>
            </div>

            <div className="space-y-2">
              <h4 className="font-medium text-blue-900">2. Start Environment</h4>
              <div className="bg-gray-900 text-green-400 p-3 rounded font-mono text-sm">
                pnpm demo:start
              </div>
              <p className="text-sm text-blue-800">
                This starts the complete StorageHub environment with blockchain node, MSP backend, and PostgreSQL database.
              </p>
            </div>

            <div className="space-y-2">
              <h4 className="font-medium text-blue-900">3. Wait for Initialization</h4>
              <p className="text-sm text-blue-800">
                The environment will initialize automatically with pre-configured MSP and BSP providers.
                This may take a few minutes on first startup.
              </p>
            </div>

            <div className="bg-yellow-50 border border-yellow-200 rounded p-3">
              <p className="text-sm text-yellow-800">
                <strong>Note:</strong> The environment runs with auto-sealing blocks every 6 seconds.
                Press Ctrl+C in the terminal to stop the environment when done.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Quick Actions */}
      {environmentReady && (
        <div className="bg-green-50 border border-green-200 rounded-lg p-4">
          <h4 className="font-medium text-green-900 mb-3">Quick Actions</h4>
          <div className="flex gap-3">
            <button type="button" className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700">
              <Play className="w-4 h-4" />
              View Logs
            </button>
            <button type="button" className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700">
              <Square className="w-4 h-4" />
              Stop Environment
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
