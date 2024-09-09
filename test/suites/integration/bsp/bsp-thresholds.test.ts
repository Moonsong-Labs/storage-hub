import assert from "node:assert";
import { bspKey, describeBspNet, type EnrichedBspApi, ferdie, sleep } from "../../../util";

describeBspNet("BSPNet: BSP Volunteering Thresholds", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("Reputation increased on successful storage", async () => {

  });
  
  // zero reputation can still volunteer and be accepted
  
  // bsp with reputation is prioritised
  
 
 // threhold globals can be changed 

 // Threshold req relaxes over blocks elapsed
 
 
});
