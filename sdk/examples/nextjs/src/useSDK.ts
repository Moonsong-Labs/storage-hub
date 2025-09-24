"use client";

import { useEffect, useState } from "react";
import { MspClient } from "@storagehub-sdk/msp-client";
import { FileManager } from "@storagehub-sdk/core";

export const useSDK = (): string => {
  const [status, setStatus] = useState<string>("Initializing SDK...");

  useEffect(() => {
    (async () => {
      const msgs: string[] = [];
      const baseUrl = "http://127.0.0.1:8080";

      // Try MSP first (short timeout so UI doesn't hang)
      try {
        const client = await MspClient.connect({ baseUrl, timeoutMs: 3000 });
        try {
          const health = await client.getHealth();
          msgs.push(`MSP Health: ${JSON.stringify(health)}`);
        } catch (e) {
          msgs.push(
            `MSP backend not reachable (${baseUrl}): ${e instanceof Error ? e.message : String(e)}`
          );
        }
      } catch (e) {
        msgs.push(`MSP init failed (${baseUrl}): ${e instanceof Error ? e.message : String(e)}`);
      }

      // Always exercise core + WASM
      try {
        const bytes = new Uint8Array([1, 2, 3, 4]);
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.enqueue(bytes);
            controller.close();
          }
        });
        const fm = new FileManager({ size: bytes.length, stream: () => stream });
        const fp = await fm.getFingerprint();
        msgs.push(`Core OK. Fingerprint: ${fp?.toHex ? fp.toHex() : String(fp)}`);
      } catch (e) {
        msgs.push(`Core error: ${e instanceof Error ? e.message : String(e)}`);
      }

      setStatus(msgs.join("\n"));
    })();
  }, []);

  return status;
};
