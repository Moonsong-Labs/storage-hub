/**
 * Unit tests for port allocation.
 */

import assert from "node:assert";
import { describe, it, beforeEach } from "node:test";
import { PortAllocator } from "../../../../util/netLaunch/dynamic/portAllocator";

describe("PortAllocator", () => {
  let allocator: PortAllocator;

  beforeEach(() => {
    allocator = new PortAllocator();
  });

  describe("Default configuration", () => {
    it("should use default base ports", () => {
      const ports = allocator.allocate("bsp", 0);

      assert.equal(ports.rpc, 9666, "RPC should start at 9666");
      assert.equal(ports.p2p, 30350, "P2P should start at 30350");
      assert.equal(ports.postgres, 5432, "Postgres should start at 5432");
    });

    it("should increment ports sequentially", () => {
      const ports1 = allocator.allocate("bsp", 0);
      const ports2 = allocator.allocate("bsp", 1);
      const ports3 = allocator.allocate("bsp", 2);

      assert.equal(ports1.rpc, 9666);
      assert.equal(ports2.rpc, 9667);
      assert.equal(ports3.rpc, 9668);

      assert.equal(ports1.p2p, 30350);
      assert.equal(ports2.p2p, 30351);
      assert.equal(ports3.p2p, 30352);

      assert.equal(ports1.postgres, 5432);
      assert.equal(ports2.postgres, 5433);
      assert.equal(ports3.postgres, 5434);
    });
  });

  describe("Global counter across node types", () => {
    it("should use global counter for all node types", () => {
      const bsp0 = allocator.allocate("bsp", 0);
      const msp0 = allocator.allocate("msp", 0);
      const fisherman0 = allocator.allocate("fisherman", 0);
      const user0 = allocator.allocate("user", 0);

      // All should use sequential ports from global counter
      assert.equal(bsp0.rpc, 9666);
      assert.equal(msp0.rpc, 9667);
      assert.equal(fisherman0.rpc, 9668);
      assert.equal(user0.rpc, 9669);
    });

    it("should not have port conflicts between node types", () => {
      const allPorts = new Set<number>();

      // Allocate ports for multiple nodes of different types
      for (let i = 0; i < 5; i++) {
        const bspPorts = allocator.allocate("bsp", i);
        const mspPorts = allocator.allocate("msp", i);

        // Check for RPC port conflicts
        assert.ok(!allPorts.has(bspPorts.rpc), `RPC port ${bspPorts.rpc} already used`);
        assert.ok(!allPorts.has(mspPorts.rpc), `RPC port ${mspPorts.rpc} already used`);
        allPorts.add(bspPorts.rpc);
        allPorts.add(mspPorts.rpc);
      }
    });
  });

  describe("Custom configuration", () => {
    it("should use custom base ports", () => {
      const customAllocator = new PortAllocator({
        rpcBase: 10000,
        p2pBase: 40000,
        postgresBase: 6000
      });

      const ports = customAllocator.allocate("bsp", 0);

      assert.equal(ports.rpc, 10000);
      assert.equal(ports.p2p, 40000);
      assert.equal(ports.postgres, 6000);
    });

    it("should increment from custom base ports", () => {
      const customAllocator = new PortAllocator({
        rpcBase: 10000
      });

      const ports1 = customAllocator.allocate("bsp", 0);
      const ports2 = customAllocator.allocate("bsp", 1);

      assert.equal(ports1.rpc, 10000);
      assert.equal(ports2.rpc, 10001);
    });
  });

  describe("Batch allocation", () => {
    it("should allocate batch of ports", () => {
      const batchPorts = allocator.allocateBatch("bsp", 5);

      assert.equal(batchPorts.length, 5, "Should allocate 5 port sets");

      // Verify sequential allocation
      for (let i = 0; i < 5; i++) {
        assert.equal(batchPorts[i].rpc, 9666 + i);
        assert.equal(batchPorts[i].p2p, 30350 + i);
        assert.equal(batchPorts[i].postgres, 5432 + i);
      }
    });

    it("should handle empty batch", () => {
      const batchPorts = allocator.allocateBatch("bsp", 0);

      assert.equal(batchPorts.length, 0, "Empty batch should return empty array");
    });
  });

  describe("Reset functionality", () => {
    it("should reset allocation counter", () => {
      allocator.allocate("bsp", 0);
      allocator.allocate("bsp", 1);
      allocator.allocate("bsp", 2);

      assert.equal(allocator.allocationCount, 3);

      allocator.reset();

      assert.equal(allocator.allocationCount, 0);

      const ports = allocator.allocate("bsp", 0);
      assert.equal(ports.rpc, 9666, "Should restart from base after reset");
    });
  });

  describe("Allocation count tracking", () => {
    it("should track allocation count", () => {
      assert.equal(allocator.allocationCount, 0);

      allocator.allocate("bsp", 0);
      assert.equal(allocator.allocationCount, 1);

      allocator.allocate("msp", 0);
      assert.equal(allocator.allocationCount, 2);

      allocator.allocateBatch("fisherman", 3);
      assert.equal(allocator.allocationCount, 5);
    });
  });

  describe("Large network support", () => {
    it("should handle 100+ nodes without conflicts", () => {
      const allRpcPorts = new Set<number>();
      const allP2pPorts = new Set<number>();

      for (let i = 0; i < 100; i++) {
        const ports = allocator.allocate("bsp", i);

        assert.ok(!allRpcPorts.has(ports.rpc), `RPC port ${ports.rpc} conflict at index ${i}`);
        assert.ok(!allP2pPorts.has(ports.p2p), `P2P port ${ports.p2p} conflict at index ${i}`);

        allRpcPorts.add(ports.rpc);
        allP2pPorts.add(ports.p2p);
      }

      assert.equal(allRpcPorts.size, 100);
      assert.equal(allP2pPorts.size, 100);
    });
  });
});
