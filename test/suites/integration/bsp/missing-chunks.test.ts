import { describeBspNet, registerToxic, type EnrichedBspApi } from "../../../util";

// TODO: Add asserts to this test case when we impl the missing chunks handling
describeBspNet(
  "BSP: Missing Chunks",
  { initialised: false, networkConfig: "noisy" },
  ({ before, it, createUserApi, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("bsp volunteers but doesn't receive chunks", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "nothingmuch-2";

      await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      //  use toxiproxy to close the connection after 50 KB
      await registerToxic({
        type: "limit_data",
        name: "limit_data",
        toxicity: 1,
        stream: "upstream",
        attributes: {
          bytes: 51200
        }
      });

      // Wait for the BSP to submit the volunteer extrinsic
      await userApi.wait.bspVolunteer();

      // Example of how to assert on a log message
      await bspApi.assert.log({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-bsp-1"
      });

      // TODO Add an assert that shows this process timing out or being handled in a specific way
    });
  }
);
