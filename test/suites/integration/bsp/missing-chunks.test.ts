import { describeBspNet, shUser, type EnrichedBspApi } from "../../../util";

describeBspNet(
  "BSP: Missing Chunks",
  { initialised: false, keepAlive: true },
  ({ before, it, createUserApi }) => {
    let userApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
    });

    it("bsp volunteers but doesn't receive ", async ()=>{
        const source = "res/whatsup.jpg";
        const destination = "test/whatsup.jpg";
        const bucketName = "nothingmuch-2";
    
        const newBucketEventEvent = await userApi.createBucket(bucketName);
        const newBucketEventDataBlob =
          userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;
    
        if (!newBucketEventDataBlob) {
          throw new Error("Event doesn't match Type");
        }
    
        const { fingerprint, file_size, location } =
          await userApi.rpc.storagehubclient.loadFileInStorage(
            source,
            destination,
            userApi.shConsts.NODE_INFOS.user.AddressId,
            newBucketEventDataBlob.bucketId
          );
    
        await userApi.sealBlock(
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
          ),
          shUser
        );
    
        await userApi.assert.extrinsicPresent({
          module: "fileSystem",
          method: "bspVolunteer",
          checkTxPool: true
        });
    })



  }
);
