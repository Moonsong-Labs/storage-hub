/**
 * Unit tests for key generation.
 */

import assert from "node:assert";
import { before, describe, it } from "node:test";
import { cryptoWaitReady } from "@polkadot/util-crypto";
import { generateNodeIdentity } from "../../../../util/netLaunch/dynamic/keyGenerator";

describe("Key generation", () => {
  // Initialize WASM crypto before running tests
  before(async () => {
    await cryptoWaitReady();
  });
  describe("generateNodeIdentity", () => {
    it("should generate BSP identity", () => {
      const identity = generateNodeIdentity("bsp", 0);

      assert.equal(identity.seed, "//BSP-Dynamic-0", "Should use correct seed format");
      assert.ok(identity.keyring, "Should have keyring");
      assert.ok(identity.keyring.address, "Keyring should have address");
      assert.ok(identity.nodeKey, "Should have node key");
      assert.ok(identity.nodeKey.startsWith("0x"), "Node key should be hex");
    });

    it("should generate MSP identity", () => {
      const identity = generateNodeIdentity("msp", 0);

      assert.equal(identity.seed, "//MSP-Dynamic-0", "Should use correct seed format");
      assert.ok(identity.keyring.address, "Keyring should have address");
    });

    it("should generate fisherman identity", () => {
      const identity = generateNodeIdentity("fisherman", 0);

      assert.equal(identity.seed, "//FISHERMAN-Dynamic-0", "Should use correct seed format");
      assert.ok(identity.keyring.address, "Keyring should have address");
    });

    it("should generate user identity", () => {
      const identity = generateNodeIdentity("user", 0);

      assert.equal(identity.seed, "//USER-Dynamic-0", "Should use correct seed format");
      assert.ok(identity.keyring.address, "Keyring should have address");
    });

    it("should generate unique seeds for different indices", () => {
      const identity0 = generateNodeIdentity("bsp", 0);
      const identity1 = generateNodeIdentity("bsp", 1);
      const identity2 = generateNodeIdentity("bsp", 99);

      assert.equal(identity0.seed, "//BSP-Dynamic-0");
      assert.equal(identity1.seed, "//BSP-Dynamic-1");
      assert.equal(identity2.seed, "//BSP-Dynamic-99");

      // Addresses should be different
      assert.notEqual(identity0.keyring.address, identity1.keyring.address);
      assert.notEqual(identity1.keyring.address, identity2.keyring.address);
    });

    it("should generate unique node keys", () => {
      const identity0 = generateNodeIdentity("bsp", 0);
      const identity1 = generateNodeIdentity("bsp", 1);

      // Node keys are random, should be different
      assert.notEqual(identity0.nodeKey, identity1.nodeKey);
    });

    it("should generate 32-byte node keys", () => {
      const identity = generateNodeIdentity("bsp", 0);

      // 0x + 64 hex chars = 66 chars total (32 bytes = 64 hex chars)
      assert.equal(identity.nodeKey.length, 66, "Node key should be 32 bytes (64 hex chars + 0x)");
    });

    it("should not have provider ID initially", () => {
      const identity = generateNodeIdentity("bsp", 0);

      assert.equal(identity.publicKey, undefined, "Public key should not be set initially");
      assert.equal(identity.providerId, undefined, "Provider ID should not be set initially");
    });

    it("should generate deterministic addresses for same seed", () => {
      const identity1 = generateNodeIdentity("bsp", 5);
      const identity2 = generateNodeIdentity("bsp", 5);

      // Same seed should produce same address
      assert.equal(
        identity1.keyring.address,
        identity2.keyring.address,
        "Same seed should produce same address"
      );
    });

    it("should generate unique addresses for different node types", () => {
      const bsp = generateNodeIdentity("bsp", 0);
      const msp = generateNodeIdentity("msp", 0);
      const fisherman = generateNodeIdentity("fisherman", 0);
      const user = generateNodeIdentity("user", 0);

      const addresses = new Set([
        bsp.keyring.address,
        msp.keyring.address,
        fisherman.keyring.address,
        user.keyring.address
      ]);

      assert.equal(addresses.size, 4, "All node types should have unique addresses");
    });
  });

  describe("Large scale identity generation", () => {
    it("should generate 100 unique identities", () => {
      const addresses = new Set<string>();
      const nodeKeys = new Set<string>();

      for (let i = 0; i < 100; i++) {
        const identity = generateNodeIdentity("bsp", i);

        assert.ok(!addresses.has(identity.keyring.address), `Address collision at index ${i}`);
        assert.ok(!nodeKeys.has(identity.nodeKey), `Node key collision at index ${i}`);

        addresses.add(identity.keyring.address);
        nodeKeys.add(identity.nodeKey);
      }

      assert.equal(addresses.size, 100, "Should have 100 unique addresses");
      assert.equal(nodeKeys.size, 100, "Should have 100 unique node keys");
    });
  });
});
