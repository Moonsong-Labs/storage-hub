import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi } from "../../../util";

await describeMspNet(
  "MSP recovers files missing in its file storage from BSPs that have them",
  { initialised: "multi", networkConfig: [{ noisy: false, rocksdb: true }] },
  ({ before, createMsp1Api, it, createUserApi }) => {
    let _userApi: EnrichedBspApi;
    let _mspApi: EnrichedBspApi;

    before(async () => {
      _userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      _mspApi = maybeMspApi;
    });

    it("Create multiple storage requests accepted by MSP and confirmed by BSPs", async () => {
      // TODO: Send multiple storage requests with replication target 3 and wait for MSP to accept and all 3 BSPs to confirm.
    });

    it("Restart MSP and check that forest roots are validated and all files are present in the forest", async () => {
      // TODO: Restart MSP and check logs for the recovery process.
    });

    it("Delete files from MSP file storage", async () => {});

    it("Restart MSP and verify files are recovered", async () => {
      // TODO: Check that MSP recovers the files from BSP one.
      // TODO: Check that BSP Two and BSP Three do not respond to the recovery request.
    });

    it("Pause BSP one and create storage request with replication target 2", async () => {
      // TODO: Pause BSP one and create a storage request with replication target 2, wait for MSP to accept and BSPs to confirm.
    });

    it("Delete new file from MSP file storage", async () => {
      // TODO: Delete a new file from MSP file storage.
    });

    it("Restart MSP and verify new file cannot be recovered from unfriendly BSPs", async () => {
      // TODO: Restart MSP and check logs for the recovery process.
      // TODO: Check that MSP does not recover the file from BSP two and BSP three.
    });
  }
);
