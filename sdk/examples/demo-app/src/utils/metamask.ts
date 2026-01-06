/**
 * Helper utilities for working with MetaMask and Ethereum providers.
 * Handles cases where multiple wallet extensions are installed.
 */

/**
 * Get MetaMask provider specifically, handling multiple wallet extensions.
 * When multiple wallets are installed, window.ethereum can be an array or have a providers array.
 */
export function getMetaMaskProvider(): EthereumProvider | null {
  if (typeof window === 'undefined') return null;
  
  try {
    const ethereum = window.ethereum;
    if (!ethereum) return null;

    // If it's an array (multiple providers), find MetaMask
    if (Array.isArray(ethereum)) {
      return ethereum.find(provider => provider.isMetaMask) || ethereum[0] || null;
    }

    // If provider has a providers array, extract MetaMask from it
    // This handles wallet picker extensions that wrap providers
    if (ethereum.providers && Array.isArray(ethereum.providers) && ethereum.providers.length > 0) {
      const metamask = ethereum.providers.find(provider => provider.isMetaMask);
      if (metamask) return metamask;
      // If no MetaMask found, use the wrapper if it's MetaMask, otherwise first provider
      return ethereum.isMetaMask ? ethereum : (ethereum.providers[0] || ethereum);
    }

    // Single provider - use it if it's MetaMask or as fallback
    if (ethereum.isMetaMask) {
      return ethereum;
    }

    // Fallback: use the provider even if we can't confirm it's MetaMask
    return ethereum;
  } catch (error) {
    console.error('[MetaMask] Error getting provider:', error);
    return null;
  }
}

/**
 * Safely make a request to the Ethereum provider with error handling.
 */
export async function safeProviderRequest(
  provider: EthereumProvider,
  method: string,
  params?: unknown[]
): Promise<unknown> {
  try {
    // If provider has a providers array and we're using the wrapper,
    // try to use the actual MetaMask provider from the array
    if (provider.providers && Array.isArray(provider.providers) && provider.providers.length > 0) {
      const actualMetaMask = provider.providers.find(p => p.isMetaMask);
      if (actualMetaMask && actualMetaMask !== provider) {
        return await actualMetaMask.request({ method, params });
      }
    }
    
    return await provider.request({ method, params });
  } catch (error) {
    // If error is from wallet picker extension, try actual MetaMask provider as fallback
    const errorMessage = error instanceof Error ? error.message : String(error);
    if ((errorMessage.includes('Unexpected error') || errorMessage.includes('selectExtension')) && 
        provider.providers && Array.isArray(provider.providers)) {
      const actualMetaMask = provider.providers.find(p => p.isMetaMask);
      if (actualMetaMask && actualMetaMask !== provider) {
        try {
          return await actualMetaMask.request({ method, params });
        } catch {
          // Fallback failed, throw original error
        }
      }
    }
    
    throw error;
  }
}

/**
 * Check if MetaMask is available
 */
export function checkMetaMaskAvailable(): boolean {
  return getMetaMaskProvider() !== null;
}
