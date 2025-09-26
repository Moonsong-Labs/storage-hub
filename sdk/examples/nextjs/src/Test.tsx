"use client";

import type { JSX } from "react";
import { useSDK } from "./useSDK";

export const Test = (): JSX.Element => {
  const status = useSDK();
  return (
    <div style={{ padding: 16 }}>
      <h1>StorageHub SDK Demo</h1>
      <pre>{status}</pre>
    </div>
  );
};
