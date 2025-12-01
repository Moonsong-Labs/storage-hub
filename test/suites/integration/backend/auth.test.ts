import assert, { strictEqual } from "node:assert";
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts";
import { describeMspNet, type EnrichedBspApi, sleep } from "../../../util";
import { BACKEND_URI } from "../../../util/backend/consts";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import { ETH_SH_USER_ADDRESS, ETH_SH_USER_PRIVATE_KEY } from "../../../util/evmNet/keyring";

const SIWE_DOMAIN = "localhost:8080";
const SIWE_URI = "http://localhost:8080";

await describeMspNet(
  "Backend bucket endpoints",
  {
    indexer: true,
    backend: true,
    runtimeType: "solochain"
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;

    let message: string;
    let signature: string;
    let token: string;

    before(async () => {
      userApi = await createUserApi();

      const maybeMsp1Api = await createMsp1Api();
      if (maybeMsp1Api) {
        msp1Api = maybeMsp1Api;
      } else {
        throw new Error("MSP API for first MSP not available");
      }
    });

    it("Postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.backend.containerName,
        searchString: "Server listening",
        timeout: 10000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Should be able to retrieve a nonce", async () => {
      const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          address: ETH_SH_USER_ADDRESS,
          chainId: SH_EVM_SOLOCHAIN_CHAIN_ID,
          domain: SIWE_DOMAIN,
          uri: SIWE_URI
        })
      });

      assert(nonceResp.ok, `Nonce request failed: ${nonceResp.status}`);
      const nonceJson = (await nonceResp.json()) as { message: string };
      message = nonceJson.message;
      assert(message, "Should receive a message from nonce endpoint");
    });

    it("Should be able to verify a signature", async () => {
      assert(message, "Should have message from previous test");

      // Sign the message from previous test
      const account = privateKeyToAccount(ETH_SH_USER_PRIVATE_KEY);
      signature = await account.signMessage({ message });

      // Verify the signature
      const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message, signature })
      });

      assert(verifyResp.ok, `Verify request failed: ${verifyResp.status}`);
      const verifyJson = (await verifyResp.json()) as { token: string };
      token = verifyJson.token;
      assert(token, "Should receive a JWT token");
    });

    it("Should not be able to verify a signature twice", async () => {
      assert(message, "Should have message from previous test");
      assert(signature, "Should have signature from previous test");

      // Try to verify the same message/signature again
      const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message, signature })
      });

      assert(!verifyResp.ok, "Second verification should fail");
      assert(verifyResp.status === 401, "Should return 401 Unauthorized");
    });

    it("Should be able to refresh a token", async () => {
      assert(token, "Should have token from previous test");

      // sleep 2 seconds to ensure timestamp changes
      await sleep(2000);

      const refreshResp = await fetch(`${BACKEND_URI}/auth/refresh`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`
        }
      });

      assert(refreshResp.ok, `Refresh request failed: ${refreshResp.status}`);
      const refreshJson = (await refreshResp.json()) as { token: string };

      assert(refreshJson.token, "Should receive a new JWT token");
      assert(refreshJson.token !== token, "New token should be different");
    });

    it("Should be able to retrieve profile", async () => {
      assert(token, "Should have token from previous test");

      const profileResp = await fetch(`${BACKEND_URI}/auth/profile`, {
        method: "GET",
        headers: {
          Authorization: `Bearer ${token}`
        }
      });

      assert(profileResp.ok, `Profile request failed: ${profileResp.status}`);
      const profileJson = (await profileResp.json()) as { address: string; ens: string };
      strictEqual(
        profileJson.address.toLowerCase(),
        ETH_SH_USER_ADDRESS.toLowerCase(),
        "Address should match"
      );
    });

    it("Should not be able to sign for another user", async () => {
      // Request nonce for ETH_SH_USER_ADDRESS
      const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          address: ETH_SH_USER_ADDRESS,
          chainId: SH_EVM_SOLOCHAIN_CHAIN_ID,
          domain: SIWE_DOMAIN,
          uri: SIWE_URI
        })
      });
      assert(nonceResp.ok, "Nonce request should succeed");
      const { message: newMessage } = (await nonceResp.json()) as { message: string };

      // Sign with a different private key
      const differentPrivateKey = generatePrivateKey();
      const differentAccount = privateKeyToAccount(differentPrivateKey);
      const wrongSignature = await differentAccount.signMessage({ message: newMessage });

      // Try to verify
      const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message: newMessage, signature: wrongSignature })
      });

      assert(!verifyResp.ok, "Verification should fail with wrong signer");
      strictEqual(verifyResp.status, 401, "Should return 401 Unauthorized");
    });

    it("Should not verify without a nonce request", async () => {
      // Create a fake message that was never issued by the backend
      const fakeMessage = "This message was never issued by the backend";

      const account = privateKeyToAccount(ETH_SH_USER_PRIVATE_KEY);
      const fakeSignature = await account.signMessage({ message: fakeMessage });

      const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message: fakeMessage, signature: fakeSignature })
      });

      assert(!verifyResp.ok, "Verification should fail without nonce request");
      strictEqual(verifyResp.status, 401, "Should return 401 Unauthorized");
    });

    it("Should reject an invalid address", async () => {
      const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          address: "not_an_eth_address",
          chainId: SH_EVM_SOLOCHAIN_CHAIN_ID,
          domain: SIWE_DOMAIN,
          uri: SIWE_URI
        })
      });

      assert(!nonceResp.ok, "Nonce request should fail with invalid address");
      strictEqual(nonceResp.status, 422, "Should return 422 Unprocessable Entity");
    });

    it("Should reject an invalid signature", async () => {
      // Get a valid nonce first
      const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          address: ETH_SH_USER_ADDRESS,
          chainId: SH_EVM_SOLOCHAIN_CHAIN_ID,
          domain: SIWE_DOMAIN,
          uri: SIWE_URI
        })
      });
      assert(nonceResp.ok, "Nonce request should succeed");
      const { message: validMessage } = (await nonceResp.json()) as { message: string };

      // Try to verify with invalid signature format
      const invalidSignature = `0x${"0".repeat(130)}`; // Wrong length for signature

      const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message: validMessage, signature: invalidSignature })
      });

      assert(!verifyResp.ok, "Verification should fail with invalid signature format");
      strictEqual(verifyResp.status, 401, "Should return 401 Unauthorized");
    });

    it.skip(
      "Should reject an expired nonce",
      { todo: "when expiry can be configured so we can run these tests in a resonable timeframe" },
      async () => {
        // Get a nonce
        const nonceResp = await fetch(`${BACKEND_URI}/auth/nonce`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            address: ETH_SH_USER_ADDRESS,
            chainId: SH_EVM_SOLOCHAIN_CHAIN_ID,
            domain: SIWE_DOMAIN,
            uri: SIWE_URI
          })
        });
        assert(nonceResp.ok, "Nonce request should succeed");
        const { message: expirableMessage } = (await nonceResp.json()) as { message: string };

        // Sign it
        const account = privateKeyToAccount(ETH_SH_USER_PRIVATE_KEY);
        const expirableSignature = await account.signMessage({ message: expirableMessage });

        // Wait for nonce to expire
        const NONCE_EXPIRY_TIME = 60000; // 60 seconds, TODO: adjust to match backend
        await sleep(NONCE_EXPIRY_TIME + 1000);

        // Try to verify after expiry
        const verifyResp = await fetch(`${BACKEND_URI}/auth/verify`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ message: expirableMessage, signature: expirableSignature })
        });

        assert(!verifyResp.ok, "Verification should fail with expired nonce");
        strictEqual(verifyResp.status, 401, "Should return 401 Unauthorized");
      }
    );
  }
);
