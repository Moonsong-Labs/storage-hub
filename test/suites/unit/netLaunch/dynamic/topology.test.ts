/**
 * Unit tests for topology validation and normalization.
 */

import assert from "node:assert";
import { describe, it } from "node:test";
import {
  normalizeTopology,
  validateTopology,
  type NetworkTopology
} from "../../../../util/netLaunch/dynamic/topology";

describe("Topology normalization", () => {
  it("should normalize number counts to config arrays", () => {
    const topology: NetworkTopology = {
      bsps: 3,
      msps: 2,
      fishermen: 1
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.bsps.length, 3, "Should have 3 BSP configs");
    assert.equal(normalized.msps.length, 2, "Should have 2 MSP configs");
    assert.equal(normalized.fishermen.length, 1, "Should have 1 fisherman config");
    assert.equal(normalized.users.length, 1, "Should default to 1 user");
  });

  it("should preserve config arrays", () => {
    const topology: NetworkTopology = {
      bsps: [{ rocksdb: true }, {}],
      msps: [{ rocksdb: true }],
      fishermen: []
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.bsps.length, 2, "Should have 2 BSP configs");
    assert.equal(normalized.bsps[0].rocksdb, true, "First BSP should have rocksdb");
    assert.equal(normalized.msps[0].rocksdb, true, "MSP should have rocksdb");
    assert.equal(normalized.fishermen.length, 0, "Should have 0 fishermen");
  });

  it("should default users to 1 when not specified", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.users.length, 1, "Should default to 1 user");
  });

  it("should handle explicit users count", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 1,
      fishermen: 0,
      users: 3
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.users.length, 3, "Should have 3 users");
  });

  it("should handle zero counts", () => {
    const topology: NetworkTopology = {
      bsps: 5,
      msps: 5,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.bsps.length, 5, "Should have 5 BSPs");
    assert.equal(normalized.msps.length, 5, "Should have 5 MSPs");
    assert.equal(normalized.fishermen.length, 0, "Should have 0 fishermen");
  });

  it("should set default collators to 1", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);

    assert.equal(normalized.collators, 1, "Collators should default to 1");
  });
});

describe("Topology validation", () => {
  it("should throw if no BSPs are specified", () => {
    const topology: NetworkTopology = {
      bsps: 0,
      msps: 1,
      fishermen: 0
    };

    assert.throws(
      () => validateTopology(topology),
      /must have at least 1 BSP/,
      "Should throw when no BSPs"
    );
  });

  it("should throw if no MSPs are specified", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 0,
      fishermen: 0
    };

    assert.throws(
      () => validateTopology(topology),
      /must have at least 1 MSP/,
      "Should throw when no MSPs"
    );
  });

  it("should throw if no users are specified", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 1,
      fishermen: 0,
      users: 0
    };

    assert.throws(
      () => validateTopology(topology),
      /must have at least 1 User/,
      "Should throw when no users"
    );
  });

  it("should accept minimal valid topology (1 BSP, 1 MSP, 1 user)", () => {
    const topology: NetworkTopology = {
      bsps: 1,
      msps: 1,
      fishermen: 0
    };

    // Should not throw (users default to 1)
    validateTopology(topology);
  });

  it("should accept complex topologies with many nodes", () => {
    const topology: NetworkTopology = {
      bsps: [{ rocksdb: true }, { rocksdb: true }, {}],
      msps: [{ capacity: 1024n * 1024n * 1024n }],
      fishermen: [{}, {}],
      users: [{}, {}, {}]
    };

    // Should not throw
    validateTopology(topology);
  });
});

describe("NodeConfig type safety", () => {
  it("should allow rocksdb configuration", () => {
    const topology: NetworkTopology = {
      bsps: [{ rocksdb: true }],
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);
    const config = normalized.bsps[0];

    assert.equal(config.rocksdb, true);
  });

  it("should allow nodes with default config", () => {
    const topology: NetworkTopology = {
      bsps: [{}],
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);
    const config = normalized.bsps[0];

    assert.equal(config.rocksdb, undefined);
    assert.equal(config.capacity, undefined);
    assert.equal(config.additionalArgs, undefined);
  });

  it("should allow custom capacity", () => {
    const capacity = 1024n * 1024n * 1024n; // 1 GB
    const topology: NetworkTopology = {
      bsps: [{ capacity }],
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);
    const config = normalized.bsps[0];

    assert.equal(config.capacity, capacity);
  });

  it("should allow additional args", () => {
    const topology: NetworkTopology = {
      bsps: [{ additionalArgs: ["--log-level=debug", "--custom-flag"] }],
      msps: 1,
      fishermen: 0
    };

    const normalized = normalizeTopology(topology);
    const config = normalized.bsps[0];

    assert.deepEqual(config.additionalArgs, ["--log-level=debug", "--custom-flag"]);
  });
});
