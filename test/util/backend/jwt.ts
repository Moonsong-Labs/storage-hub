import assert from "node:assert";
import { privateKeyToAccount } from "viem/accounts";
import { BACKEND_URI } from "./consts";

// Perform SIWE auth flow using the backend's endpoints to generate a JWT token
export async function fetchJwtToken(privateKey: `0x${string}`, chainId: number): Promise<string> {
  // Create an account from the received private key
  const account = privateKeyToAccount(privateKey);

  // Fetch a nonce from the backend for the given account and chainId
  const siweDomain = "localhost:8080";
  const siweUri = "http://localhost:8080";
  const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ address: account.address, chainId, domain: siweDomain, uri: siweUri })
  });
  assert(nonceResp.ok, `Nonce request failed: ${nonceResp.status}`);
  const { message } = (await nonceResp.json()) as { message: string };

  // Sign the message with the user's Ethereum key
  const signature = await account.signMessage({ message });

  // Verify the signature and generate the JWT token
  const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message, signature })
  });
  assert(verifyResp.ok, `Verify request failed: ${verifyResp.status}`);
  const verifyJson = (await verifyResp.json()) as { token: string };
  return verifyJson.token;
}
