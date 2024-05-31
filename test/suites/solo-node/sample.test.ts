import { expect, test, describe, beforeAll } from "bun:test";
import { GenericContainer, type StartedTestContainer } from "testcontainers";

// TODO: When storage-hub can be run as a solo node with --dev, we can add tests here

describe("Sample test suite", () => {
  let container: StartedTestContainer;

  beforeAll(async () => {
    container = await new GenericContainer("paritytech/polkadot:latest")
      .withExposedPorts(9944)
      .start();
  });

  test("2 + 2", () => {
    expect(2 + 2).toBe(4);
  });

  test("truthy", () => {
    expect(1 === Number("1")).toBeTrue;
  });
});
