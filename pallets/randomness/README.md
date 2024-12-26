# Randomness

This pallet provides access to randomness using as source the **BABE epoch randomness**, which is produced by the relay chain per relay chain epoch. It is based on **all the VRF produced** by the relay chain validators **during** a complete **epoch**.(~600 blocks on Kusama, ~2400 blocks on Polkadot). At the beginning of a new Epoch, those VRFs are **mixed together** and **hashed** in order to produce a **pseudo-random word**.

## Babe Epoch Randomness

### Properties

- This randomness is totally **independent of the parachain**, preventing a malicious actor on the parachain to influence the randomness value.
- This randomness is **constant during a full epoch range** (~250 blocks on Kusama, ~2300 blocks on Polkadot) making it **resilient enough against censorship**. If a collator prevents fulfillment at a given block, another collator can fulfill it at the next block with the same random value.
- This randomness **requires** at least 1 epoch after the current epoch (**~1h30** on Kusama, **~6h** on Polkadot) to ensure the pseudo-random word cannot be predicted at the time of the request.

### Risks

The **danger** in this process comes from the knowledge that the **last validator** (Validator Y in the schema) has when producing the last block of an Epoch. The process being deterministic and all the material to generate the pseudo random word being known, the validator can decide to **skip producing the block** in order to not include its VRF, which would result in a different pseudo-random word.

Because epochs are time-based, if the block is skipped, there won't be any additional block produced for that epoch. So the last validator of the block knows both possible output:

1. When **producing the block** including its VRF => pseudo-random word **AAAA**
2. When **skipping the block** and using already known previous VRFs => pseudo-random word **BBBB**

The only **incentive** to prevent the validator from skipping the block is the **block rewards**. So the randomness value is only **economically safe if the value at stake is lower than a block reward**.

```sequence
note over Validator: Validator A
note over Relay: Epoch 1: Block #2399
Relay->Para: (Relay Block #2399)
note over Para: Block #111\nRequest Randomness (@Epoch 3)
note left of Relay: No knowledge of epoch 2 randomness\nexists yet
Validator->Relay: (Relay Block #2400)
note over Relay: Epoch 2: Block #2400\n(random epoch 1: 0xAAAAAA...)
note over Relay: .\n.\n.
note over Para: .\n.\n.
note over Validator: Validator X
Validator->Relay: Produces #4798\n(influences Epoch 2 Randomness\nbut doesn't know the result)
note over Validator: Validator Y
Validator->Relay: Produces #4799\n(knows/influences Epoch 2 Randomness)\ncan choose 0xBBBBBB... or 0xCCCCCC...
note over Relay: Epoch 3: Block #4800\n(random epoch 2: 0xBBBBBB...or 0xCCCCCC...)
Relay->Para: (Relay Block #4800)
note over Para: Block #222\nFulFill Randomness using\n0xBBBBBB...or 0xCCCCCC...
```

_In this schema, we can see that validator Y can decide the epoch 2 randomness by producing or skipping its block._

### Multiple slot leaders

Additionally, the Babe consensus can sometimes allow multiple validator to produce a block at the same slot. If that is the last slot of an Epoch,the selected validators coordinate in order to decide which one is producing the block, offering the choice of even more pseudo-random words.

### Asynchronous Backing

This solution is **safe** even after the asynchronous backing is supported as the pseudo-random is not dependent on which relay block the parachain block is referencing.
A collator being able to choose the relay block on top of which it builds the parachain block will not influence the pseudo-random word.
