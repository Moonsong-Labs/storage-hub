# Storage Providers Pallet

## Overview

The Storage Providers pallet is designed for Substrate-based blockchains, providing a robust framework for managing storage providers within a decentralized network. This pallet allows for the registration and management of Main Storage Providers (MSPs) and Backup Storage Providers (BSPs), facilitating operations related to their sign-up, capacity management, and service offerings.

### Features

- Provider Management: Enables the sign-up of MSPs and BSPs, allowing them to register and become part of the network's storage solution.
- Capacity and Value Proposition Management: Providers can modify their storage capacities and update their value propositions to reflect their current offerings and capabilities.
- Lifecycle Events: Tracks significant lifecycle events like sign-up requests, confirmations, and sign-offs through a series of emitted events, ensuring transparency and traceability.
- Robust Access Controls: Ensures that only authorized operations are performed by storage providers, enhancing security and operational integrity.

### Target Audience

This pallet is intended for blockchain developers interested in integrating decentralized storage solutions into their Substrate-based blockchain. It provides essential services for managing the roles and capabilities of storage providers within the ecosystem. It was developed with StorageHub's specific providers framework but can be used by any other network that fits the Main Storage Provider/Backup Storage Provider structure.

## Extrinsics

### request_msp_sign_up

The purpose of this extrinsic is to handle the sign-up process for a new Main Storage Provider. It performs several checks and updates the blockchain's storage accordingly. This extrinsic:

1) Verifies that registering this new Main Storage Provider will not exceed the maximum limit of Main Storage Providers allowed by the runtime.
2) Verifies that the signer is not already registered as either a Main Storage Provider or Backup Storage Provider.
3) Validates the provided multiaddress, which represents the network address where the storage data will be accessible for the Main Storage Provider.
4) Ensures that the capacity that the user wants to register with is greater than the minimum required by the runtime.
5) Calculates the deposit amount that the signer needs to pay based on the desired storage capacity, checks that the signer has enough funds to cover it and
holds the deposit of the signer.
6) Updates the storage to add the signer as requesting to sign up as a Main Storage Provider
7) Emits an event to confirm the successful sign-up request as an Main Storage Provider.

#### Parameters

The parameters that this extrinsic accepts are:

- `origin`: The origin of the transaction, which should be a signed origin. This is the account ID of the runtime that is requesting to sign up as a Storage Provider.
- `capacity`: The capacity with which the signer of the transaction wants to register with.
- `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the [Multiaddr spec](https://github.com/multiformats/multiaddr)).
- `value_prop`: The value proposition that the signer will provide as a Main Storage Provider to users and wants to register on-chain. It could be data limits, communication protocols to access the user's data.

#### Example

Alice wants to register as a Main Storage Provider:

```rust
let alice_multiaddress = vec!["/ip4/127.0.0.1/udp/1234".as_bytes().to_vec().try_into().unwrap()]
let alice_value_prop = ValueProposition {
                        identifier: 0,
                        data_limit: 10,
                        protocols: BoundedVec::new(),
                    }
request_msp_sign_up(origin: RuntimeOrigin::signed(alice), capacity: 100, multiaddresses: alice_multiaddress, value_prop: alice_value_prop);
```

### request_bsp_sign_up

The purpose of this extrinsic is to handle the sign-up process for a new Backup Storage Provider. It performs several checks and updates the blockchain's storage accordingly. This extrinsic:

1) Verifies that registering this new Backup Storage Provider will not exceed the maximum limit of Backup Storage Providers allowed by the runtime.
2) Verifies that the signer is not already registered as either a Main Storage Provider or Backup Storage Provider.
3) Validates the provided multiaddress, which represents the network address where the storage data will be accessible for the Backup Storage Provider.
4) Ensures that the capacity that the user wants to register with is greater than the minimum required by the runtime.
5) Calculates the deposit amount that the signer needs to pay based on the desired storage capacity, checks that the signer has enough funds to cover it and
holds the deposit of the signer.
6) Updates the storage to add the signer as requesting to sign up as a Backup Storage Provider
7) Emits an event to confirm the successful sign-up request as an Backup Storage Provider.

#### Parameters

The parameters that this extrinsic accepts are:

- `origin`: The origin of the transaction, which should be a signed origin. This is the account ID of the runtime that is requesting to sign up as a Storage Provider.
- `capacity`: The capacity with which the signer of the transaction wants to register with.
- `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the [Multiaddr spec](https://github.com/multiformats/multiaddr))

#### Example

Alice wants to register as a Backup Storage Provider:

```rust
let alice_multiaddress = vec!["/ip4/127.0.0.1/udp/1234".as_bytes().to_vec().try_into().unwrap()]
request_bsp_sign_up(origin: RuntimeOrigin::signed(alice), capacity: 100, multiaddresses: alice_multiaddress);
```

### confirm_sign_up

The purpose of this extrinsic is to allow users to confirm their sign up as a Storage Provider (be it a Main Storage Provider or a Backup Storage Provider )after the required time has passed to allow the runtime's randomness used for this process to not have been predictable when the sign up request was made.This extrinsic:

1. Verifies that the account received has requested to register as a Storage Provider.
2. Ensures that by registering this Storage Provider we would not go over the limit of Main Storage Providers or Backup Storage Providers.
3. Check that the current randomness of the runtime was not predictable when the sign up request was registered.
4. Check that the request has not expired.
5. Use the current randomness obtained from the runtime as salt to generate the unique ID of the new Storage Provider.
6. Register the signer as a Main Storage Provider or Backup Storage Provider with the metadata provided in the initial request.
7. Emit an event confirming that the sign up of the SP has been completed.

Notes:

- This extrinsic could be called by the user that requested the registration itself or by a third party in behalf of the user.
- Requests have an expiration because if that wasn't the case, malicious users could wait indefinitely for a random seed from the relay chain that suits their malicious purpose.
- The deposit that the user has to pay to register as a Storage Provider is held when the user requests to register as a Storage Provider, not in this extrinsic.
- If this extrinsic is successful, it will be free for the caller, to incentive state debloating of pending requests.

#### Parameters

The parameters that this extrinsic accepts are:

- `origin`: The origin of the transaction, which should be a signed origin.
- `provider_account`: An optional parameter, used in cases where the origin of the transaction is not the account that initiated the request to sign up as a Storage Provider.

#### Example

Bob wants to confirm the sign up of Alice as a Storage Provider:

```rust
confirm_sign_up(RuntimeOrigin::signed(bob), alice);
```

### cancel_sign_up

The purpose of this extrinsic is to allow users to cancel their sign up request that they previously initiated. This extrinsic:

1) Ensures that the signer (or the optional `provider_account` if present) has previously requested to sign up as a Storage Provider.
2) Deletes the request from the Sign Up Requests storage.
3) Returns the previously held deposit to the signer.
4) Emits an event confirming that the cancellation of the sign up request has been successful.

Notes:

- Since requesting to sign up already holds the funds of the deposit from the user, this must be called to recover it. This way, we incentivize storage debloat as users will want to delete the sign up requests that are not going to be confirmed.

#### Parameters

The parameter that this extrinsic accepts is:

- `origin`: The origin of the transaction, which must be a user with a pending storage request.

#### Example

Alice mistakenly tried to register with a different capacity that what she wanted, so she cancels the request:

```rust
cancel_sign_up(RuntimeOrigin::signed(alice));
```

### msp_sign_off

The purpose of this extrinsic is to allow Main Storage Providers that are not currently being used by any user to sign off (deregister) as a Storage Provider and recover their deposit. This extrinsic:

1) Verifies that the signer is registered as a Main Storage Provider.
2) Ensures that the Main Storage Provider has no user storage assigned to it (no buckets or data in use).
3) Updates the Main Storage Provider's metadata storage, removing the signer as a Main Storage Provider.
4) Returns the deposit to the signer.
5) Decrements the storage that holds total amount of Main Storage Providers currently in the system
6) Emits an event confirming that the sign off of the Main Storage Provider has been successful

#### Parameters

- `origin`: The origin of the transaction, which must be a user registered as a Main Storage Provider.

#### Example

Alice is no longer providing storage to users so she wants to recover its deposit:

```rust
msp_sign_off(RuntimeOrigin::signed(alice));
```

### bsp_sign_off

The purpose of this extrinsic is to allow Main Storage Providers that are not currently being used by any user to sign off (deregister) as a Storage Provider and recover their deposit. This extrinsic:

1) Verifies that the signer is registered as a Backup Storage Provider.
2) Ensures that the Backup Storage Provider has no user storage assigned to it (no data in use).
3) Update the total capacity of the network (which is the capacity of all Backup Storage Providers), subtracting the capacity of this Backup Storage Provider.
4) Updates the Backup Storage Provider's metadata storage, removing the signer as a Backup Storage Provider.
5) Returns the deposit to the signer.
6) Decrements the storage that holds total amount of Backup Storage Providers currently in the system.
7) Emits an event confirming that the sign off of the Backup Storage Provider has been successful

#### Parameters

- `origin`: The origin of the transaction, which must be a user registered as a Backup Storage Provider.

#### Example

Alice is no longer providing storage to the network so she wants to recover its deposit:

```rust
bsp_sign_off(RuntimeOrigin::signed(alice));
```

### change_capacity

The purpose of this extrinsic is to allow Storage Providers (Main or Backup) to change their "contracted" capacity, increasing or decreasing it as they see fit. The new capacity has to be more than the minimum allowed by the runtime, more than the Storage Provider's used capacity and the change is subject to a timelock to avoid spam attacks. This extrinsic:

1) Verifies that the signer is registered as a Storage Provider.
2) Ensures that enough time has passed since the last time the Storage Provider changed its capacity (timelock).
3) Ensures that the new capacity is greater than the minimum required by the runtime.
4) Ensures that the new capacity is greater than the data that this Storage Provider has as used.
5) Calculates the new deposit needed for this new capacity.
6) Checks to see if the new deposit needed is greater or less than the current deposit.
    a) If the new deposit is greater than the current deposit:
        i) Ensures that the signer has enough funds to pay this extra deposit.
        ii) Holds the extra deposit from the signer.
    b) If the new deposit is less than the current deposit:
        i) Returns the held difference to the signer.
7) Updates the Storage Provider's metadata storage to change the total data.
8) If the user is a Backup Storage Provider, it updates the total capacity of the network.
9) Emits an event confirming that the change of the capacity has been successful.

#### Parameters

- `origin`: The origin of the transaction, which must be a user registered as a Storage Provider (Main or Backup).
- `new_capacity`: The new capacity that the Storage Provider now wants to provide to the users/network.

#### Example

Alice is providing 100TBs and has improved her infrastructure, so now wants to provide 200TBs:

```rust
change_capacity(RuntimeOrigin::signed(alice), 200);
```

Bob is providing 100TBs but is looking to scale down so wants to reduce his available storage to 50TBs:

```rust
change_capacity(RuntimeOrigin::signed(bob), 50);
```

### add_value_prop

The purpose of this extrinsic is to allow Main Storage Providers to add new value propositions to their offerings. This allows them to offer service tiers to their users, with different fee structures and features.

#### Parameters

- `origin`: The origin of the transaction, which must be a user registered as a Main Storage Provider.
- `new_value_prop`: The new value proposition to add to the Main Storage Provider list of offered value propositions.

#### Example

Alice is a Main Storage Provider with a single tier (value proposition) that has a per-user limit of 10TBs of storage. She now wants to offer a premium tier that has a per-user limit of 100TBs of storage:

```rust
let new_tier: ValueProposition<T> = ValueProposition {
    identifier: 1,
    data_limit: 100,
    protocols: BoundedVec::new(),
};
add_value_prop(RuntimeOrigin::signed(alice), new_tier);
```

## Interfaces

This pallet implements the following interfaces:

- `MutateProvidersInterface`
- `ReadProvidersInterface`
- `ProvidersInterface`

These are further explained in their own documentation.

## Storage

### `SignUpRequests`

This storage holds the sign up requests initiated by users of StorageHub that want to offer their services as Storage Providers, both Main and Backup.
It is updated in:

- `request_msp_sign_up` and `request_bsp_sign_up`, which add a new entry to the map.
- `confirm_sign_up` and `cancel_sign_up`, which remove an existing entry from the map.

#### Fields

It's a map from an account ID to a tuple consisting of the Storage Provider metadata that the account used when requesting to sign up and the block number in which the request was initiated.

```
AccountId -> (StorageProviderMetadata, BlockWhenSignUpWasRequested)
```

### `AccountIdToMainStorageProviderId`

This storage is used to keep track of the one-to-one relationship between an account ID and a Main Storage Provider ID, which is used to choose which challenges are requested from that Storage Provider. It is updated in:

- `confirm_sign_up`, which adds a new entry to the map if the account to confirm is a Main Storage Provider.
- `msp_sign_off`, which removes the corresponding entry from the map.

#### Fields

It's a map from an account ID to a Main Storage Provider ID, which is of the Hash type from the runtime.

```
AccountId -> MainStorageProviderId
```

### `MainStorageProviders`

This storage holds the metadata of each registered Main Storage Provider, including its corresponding buckets, its capacity, used data, the valid multiaddresses to connect to it, its list of value propositions and the block in which this Storage Provider last changed its capacity. It is updated in:

- `confirm_sign_up`, which adds a new entry to the map if the account to confirm is a Main Storage Provider.
- `msp_sign_off`, which removes the corresponding entry from the map.
- `change_capacity`, which changes the entry's `capacity`.
- `add_value_prop`, which appends a new value proposition to the entry's existing `value_prop` bounded vector.

#### Fields

It's a map from a Main Storage Provider ID to its metadata.

```
MainStorageProviderId -> MainStorageProviderMetadata
```

### `Buckets`

This storage holds the metadata of each bucket that exists in each Main Storage Provider. It holds the bucket's root, the user ID that owns that bucket and the Main Storage Provider ID that holds that bucket. It is updated using the `MutateProvidersInterface`, by the functions:

- `add_bucket`, which adds a new entry to the map.
- `change_root_bucket`, which changes the corresponding bucket's root.
- `remove_root_bucket`, which removes the entry of the corresponding bucket.

#### Fields

It's a map from a bucket ID to that bucket's metadata

```
BucketId -> Bucket
```

### `AccountIdToBackupStorageProviderId`

This storage is used to keep track of the one-to-one relationship between an account ID and a Backup Storage Provider ID, which is used to both choose which challenges are requested from that Storage Provider and to compare with the threshold used to allow Backup Storage Providers to offer themselves to store a new file of the system. It is updated in:

- `confirm_sign_up`, which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
- `bsp_sign_off`, which removes the corresponding entry from the map.

#### Fields

It's a map from an account ID to a Backup Storage Provider ID, which is of the Hash type from the runtime.

```
AccountId -> BackupStorageProviderId
```

### `BackupStorageProviders`

This storage holds the metadata of each registered Backup Storage Provider, which has its capacity, its used data, the valid multiaddresses to connect to it, its forest root and the block in which this Storage Provider last changed its capacity. It is updated in:

- `confirm_sign_up`, which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
- `bsp_sign_off`, which removes the corresponding entry from the map.
- `change_capacity`, which changes the entry's `capacity`.

#### Fields

It's a map from a Backup Storage Provider ID to its metadata.

```
BackupStorageProviderId -> BackupStorageProviderMetadata
```

### `MspCount`

This storage holds the amount of Main Storage Providers that are currently registered in the system. It is updated in:

- `confirm_sign_up`, which adds one to this storage if the account to confirm is a Main Storage Provider.
- `msp_sign_off`, which subtracts one from this storage.

### `BspCount`

This storage holds the amount of Backup Storage Providers that are currently registered in the system. It is updated in:

- `confirm_sign_up`, which adds one to this storage if the account to confirm is a Backup Storage Provider.
- `bsp_sign_off`, which subtracts one from this storage.

### `TotalBspsCapacity`

This storage holds the sum of all the capacity that has been registered by Backup Storage Providers, which corresponds to the capacity of the whole network. It is updated in:

- `confirm_sign_up`, which adds the capacity of the registered Storage Provider to this storage if the account to confirm is a Backup Storage Provider.
- `bsp_sign_off`, which subtracts the capacity of the Backup Storage Provider to sign off from this storage.

## Events

### `MspRequestSignUpSuccess`

This event is emitted when a Main Storage Provider has requested to sign up successfully. It provides information about that Main Storage Provider's account ID, the list of valid multiaddresses that it wants to register, the total capacity that it wants to register, and its list of value propositions.

```rust
MspRequestSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
    value_prop: ValueProposition<T>,
}
```

### `MspSignUpSuccess`

This event is emitted when a Main Storage Provider has confirmed its requested sign up successfully. It provides information about that Main Storage Provider's account ID, the list of valid multiaddresses that it has registered, the total capacity that it has registered, and its list of value propositions.

```rust
MspSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
    value_prop: ValueProposition<T>,
}
```

### `BspRequestSignUpSuccess`

This event is emitted when a Backup Storage Provider has requested to sign up successfully. It provides information about that Backup Storage Provider's account ID, the list of valid multiaddresses that it wants to register and the total capacity that it wants to register.

```rust
BspRequestSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
}
```

### `BspSignUpSuccess`

This event is emitted when a Backup Storage Provider has confirmed its requested sign up successfully. It provides information about that Backup Storage Provider's account ID, the list of valid multiaddresses that it has registered and the total capacity that it has registered.

```rust
BspSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
}
```

### `SignUpRequestCanceled`

This event is emitted when a Storage Provider has cancelled its request to sign up successfully and the previously held deposit has been returned to it.

```rust
SignUpRequestCanceled { who: T::AccountId }
```

### `MspSignOffSuccess`

This event is emitted when a Main Storage Provider has signed off of the system successfully and the previously held deposit has been returned to it.

```rust
MspSignOffSuccess { who: T::AccountId }
```

### `BspSignOffSuccess`

This event is emitted when a Backup Storage Provider has signed off of the system successfully and the previously held deposit has been returned to it.

```rust
BspSignOffSuccess { who: T::AccountId }
```

### `CapacityChanged`

This event is emitted when a  Storage Provider has successfully changed its registered capacity on the network. It holds the information about the account ID of that Storage Provider, its previous capacity, the new registered capacity and the next block after which the timelock expires and it is able to change its capacity again.

```rust
CapacityChanged {
    who: T::AccountId,
    old_capacity: StorageData<T>,
    new_capacity: StorageData<T>,
    next_block_when_change_allowed: BlockNumberFor<T>,
}
```

## Error

### `AlreadyRegistered`

Error thrown when a user tries to sign up as a Storage Provider but is already registered as either a Main Storage Provider or Backup Storage Provider.

### `MaxBspsReached`

Error thrown when a user tries to sign up as a Backup Storage Provider but the maximum amount of Backup Storage Providers has been reached.

### `MaxMspsReached`

Error thrown when a user tries to sign up as a Main Storage Provider but the maximum amount of Main Storage Providers has been reached.

### `SignUpNotRequested`

Error thrown when a user tries to confirm a sign up that was not requested previously.

### `SignUpRequestPending`

Error thrown when a user tries to request to sign up when it already has a sign up request pending.

### `NoMultiAddress`

Error thrown when a user tries to sign up providing an empty list of multiaddresses.

### `InvalidMultiAddress`

Error thrown when a user tries to sign up as a Storage Provider but any of its provided multiaddresses is invalid.

### `StorageTooLow`

Error thrown when a user tries to sign up or change its capacity to a value smaller than the minimum required by the runtime.

### `NotEnoughBalance`

Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a Storage Provider or changing its capacity to one that entails a bigger deposit.

### `CannotHoldDeposit`

Error thrown when the runtime cannot hold the required deposit from the account to register it as a Storage Provider or change its capacity.

### `StorageStillInUse`

Error thrown when a user tries to sign off as a Storage Provider but still has storage that's not free.

### `RandomnessNotValidYet`

Error thrown when a user tries to confirm a sign up but the available randomness from the runtime could have still been predicted by the user that requested the sign up.

### `SignUpRequestExpired`

Error thrown when a user tries to confirm a sign up but too much time has passed since it initially requested to sign up.

### `NewCapacityLessThanUsedStorage`

Error thrown when a user tries to change its capacity to less than the capacity that is has used.

### `NewCapacityEqualsCurrentCapacity`

Error thrown when a user tries to change its capacity to the same value it already has.

### `NewCapacityCantBeZero`

Error thrown when a user tries to change its capacity to zero (there are specific extrinsics to sign off as a Storage Provider).

### `NotEnoughTimePassed`

Error thrown when a Storage Provider tries to change its capacity but it has not been enough time since the last time it changed it, so the timelock is still active.

### `NotRegistered`

Error thrown when a user tries to interact as a Storage Provider with this pallet but it is not registered as either a Main Storage Provider or a Backup Storage Provider.

### `NoUserId`

Error thrown when trying to get the root of a bucket that belongs to a Main Storage Provider without passing a user ID needed to identify that bucket.

### `NoBucketId`

Error thrown when trying to get a root from a Main Storage Provider without passing the bucket ID of the bucket that the root should belong to.

### `SpRegisteredButDataNotFound`

Error thrown when a user has a Storage Provider ID assigned to it but its metadata data does not exist in storage (storage inconsistency error, should never happen).
