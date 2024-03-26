import { expect, test, describe } from "bun:test";
import { sendRPCRequest } from "../../util";

describe("Basic Parachain Node RPC tests", () => {
  test("Check type", async () => {
    const response = await sendRPCRequest("system_chainType");
    console.log(response);
    expect(response.status).toBe(200);
  });

  test("Check name", async () => {
    const response = await sendRPCRequest("system_name");
    console.log(response);
    expect(response.status).toBe(200);
  });

  test("Check Properties", async () => {
    const response = await sendRPCRequest("system_properties");
    console.log(response);
    expect(response.status).toBe(200);
  });
});
