import { describe, it, before } from "node:test";
import { expect } from "expect";
import { GenericContainer, type StartedTestContainer } from "testcontainers";

// TODO: When storage-hub can be run as a solo node with --dev, we can add tests here

describe("Sample test suite", () => {
  let container: StartedTestContainer;

  before(async () => {
    // container = await new GenericContainer("paritytech/polkadot:latest")
    //   .withExposedPorts(9944)
    //   .start();
  });

  it("2 + 2", () => {
    // console.log("Container IP: ", container.getHost());
    expect(2 + 2).toBe(4);
  });

  it("truthy", () => {
    expect(1 === Number("1")).toBe(true);
  });
});
