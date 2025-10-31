declare global {
  // Allow process.env usage in the browser build for env-inlined values
  // This avoids pulling full Node types and keeps typing minimal
  // Only include what we actually read
  namespace NodeJS {
    interface ProcessEnv {
      NEXT_PUBLIC_APP_CONFIG_FILE?: string;
    }
  }
  const process: { env: NodeJS.ProcessEnv };
  interface Window {
    ethereum?: {
      request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
      on: (event: string, handler: (...args: unknown[]) => void) => void;
      removeListener: (event: string, handler: (...args: unknown[]) => void) => void;
      isMetaMask?: boolean;
    };
  }
}

export { };
