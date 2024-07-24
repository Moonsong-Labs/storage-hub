# Storage Providers Pallet

## Overview

The Storage Providers pallet is designed for Substrate-based blockchains, providing a robust framework for managing storage providers within a decentralized network. This pallet allows for the registration and management of Main Storage Providers (MSPs) and Backup Storage Providers (BSPs).
It is designed to be flexible and extensible, allowing developers to integrate it with decentralized storage solutions in their blockchain networks with ease. It provides essential services for managing the roles and capabilities of storage providers within the ecosystem, ensuring transparency, security, and operational integrity.

### Features

- Provider Management: Enables the sign-up of MSPs and BSPs, allowing them to register and become part of the network's storage solution.
- Capacity and Value Proposition Management: Providers can modify their storage capacities and update their value propositions to reflect their current offerings and capabilities.
- Lifecycle Events: Tracks significant lifecycle events like sign-up requests, confirmations, and sign-offs through a series of emitted events.
- Robust Access Controls: Ensures that only authorized operations are performed by authorized storage providers.
- Error Handling: Provides detailed error messages to help developers understand and resolve issues during storage provider management operations.

### Target Audience

This pallet is intended for blockchain developers interested in integrating decentralized storage solutions into their Substrate-based blockchain. It provides essential services for managing the roles and capabilities of storage providers within the ecosystem. It was developed with StorageHub's specific providers framework but can be used by any other network that fits the Main Storage Provider/Backup Storage Provider structure, with some modifications.

## Design

### Main Storage Providers (MSPs)

A Main Storage Provider (MSP) is responsible for storing the complete file provided by the user and making it readily available for off-chain retrieval. MSPs are expected to compete for user adoption, and a certain degree of centralization is anticipated. They are considered trusted parties for data retrieval.

MSPs are compensated for their services by the user. The amount is agreed upon when the user selects the MSP, and it's typically higher than what a Backup Storage Provider receives due to the infrastructure required for convenient data retrieval.

MSPs are required to regularly prove that they continue to store the data they've committed to. This ensures the integrity and availability of the data in the system.

### Backup Storage Providers (BSPs)

Backup Storage Providers (BSPs) store a portion of the user's file and must have it readily available to be served to another Storage Provider if the user designates a new MSP. Unlike MSPs, BSPs do not need to make the data available for public retrieval.

BSPs play a crucial role in the system as a fallback option if the trust with the MSP is broken. They ensure that the user can freely choose another MSP, with the assurance that their data is still available in the Storage Hub.

BSPs do not compete for user adoption as they do not offer distinctive services to one another and are not chosen by the user. Instead, they are assigned by the system, with considerations to ensure an even distribution of data.

The remaining portion of the user's payment for storing a file, after the MSP's share, is evenly distributed among the assigned BSPs. This incentivizes BSPs to maintain the data they've committed to storing.

### Sign Up Process

The sign up process for Storage Providers is a two-step process to avoid malicious users from predicting the randomness used to generate the unique ID of the Storage Provider.

1. The first step is the request to sign up, where the user provides the necessary information to become a Storage Provider, commiting to that information, and the necessary deposit is held.
2. The second step is the confirmation of the sign-up, which must be done by the user (or a third-party) after enough time has passed to ensure that the randomness used to generate the unique ID was not predictable when the sign-up request (commitment) was made.

This process exists because the unique ID of a Storage Provider is what determines if they can volunteer to store a new file of the system after a store request was made by a user, and if the randomness used to generate the unique ID was predictable, malicious users could generate multiple Storage Provider accounts with similar IDs and collude to store the same file, which would be detrimental to the system as it would allow file storage centralization, allowing censorship and data loss.

### Sign Off Process

The sign off process is simpler that the sign up process: the Storage Provider requests to sign off, and if it does not have any data currently in use (that means, no user file is currently being stored by this Storage Provider), the deposit is returned and the Storage Provider information is completely deleted from the system. This deletion is permanent, as the unique ID of the Storage Provider is generated using the runtime's randomness, and it is not possible to recreate it.

### Capacity Management

Storage Providers can change their capacity, increasing or decreasing it as they see fit. The new capacity has to be more than the minimum allowed by the runtime, more than the Storage Provider's used capacity, and the change is subject to a timelock to avoid spam attacks. The new deposit needed for the new capacity is calculated, and the user has to pay the difference if the new deposit is greater than the current deposit. If the new deposit is less than the current deposit, the held difference is returned to the user. This allows Storage Providers to adapt to the network's needs and their own infrastructure capabilities.

## Extrinsics

The Storage Providers pallet provides the following extrinsics, which are explained at a high level in this section. For detailed information on the parameters and usage of each extrinsic, refer to the documentation found in the code.

### request_msp_sign_up

The purpose of this extrinsic is to handle the sign-up process for a new Main Storage Provider. It performs several checks and updates the blockchain's storage accordingly. We have a two-step process for signing up as a Storage Provider to avoid malicious users from predicting the randomness used to generate the unique ID of the Storage Provider, and that's why we have a request and a confirmation extrinsic, as a commitment scheme.

### request_bsp_sign_up

The purpose of this extrinsic is to handle the sign-up process for a new Backup Storage Provider. It performs several checks and updates the blockchain's storage accordingly. This extrinsic is similar to the `request_msp_sign_up` extrinsic, but for Backup Storage Providers.

### confirm_sign_up

The purpose of this extrinsic is to allow users to confirm their sign up as a Storage Provider (be it a Main Storage Provider or a Backup Storage Provider) after the required time has passed to allow the runtime's randomness used for this process to not have been predictable when the sign up request was made. This extrinsic is only available for users that have a pending sign up request.

Notes:

- This extrinsic could be called by the user that requested the registration itself or by a third party in behalf of the user.
- Requests have an expiration because if that wasn't the case, malicious users could wait indefinitely for a random seed from the relay chain that suits their malicious purpose.
- The deposit that the user has to pay to register as a Storage Provider is held when the user requests to register as a Storage Provider, not in this extrinsic.
- If this extrinsic is successful, it will be free for the caller, to incentive state debloating of pending requests.

### cancel_sign_up

The purpose of this extrinsic is to allow users to cancel their sign up request that they previously initiated. This allows users to recover the deposit that was held when they requested to sign up as a Storage Provider, and it is a way to incentivize storage debloat as users will want to delete the sign up requests that are not going to be confirmed. This extrinsic is only available for users that have a pending sign up request.

### msp_sign_off

The purpose of this extrinsic is to allow Main Storage Providers that are not currently being used by any user to sign off (deregister) as a Storage Provider and recover their deposit. This extrinsic is only available for Main Storage Providers that have no user storage assigned to them (no data in use). We have this restriction to avoid data loss, as if a Main Storage Provider has data in use and signs off, the data would be lost.

### bsp_sign_off

The purpose of this extrinsic is to allow Backup Storage Providers that are not currently being used by any user to sign off (deregister) as a Storage Provider and recover their deposit. This extrinsic is only available for Backup Storage Providers that have no user storage assigned to them (no data in use). The logic is the same as the `msp_sign_off` extrinsic, but for Backup Storage Providers.

### change_capacity

The purpose of this extrinsic is to allow Storage Providers (Main or Backup) to change their "contracted" capacity, increasing or decreasing it as they see fit. The new capacity has to be more than the minimum allowed by the runtime, more than the Storage Provider's used capacity and the change is subject to a timelock to avoid spam attacks. This extrinsic is available for all registered Storage Providers.

### add_value_prop

The purpose of this extrinsic is to allow Main Storage Providers to add new value propositions to their offerings. This allows them to offer service tiers to their users, with different fee structures and features.

## Interfaces

This pallet implements the following interfaces:

- `MutateProvidersInterface`
- `ReadProvidersInterface`
- `ProvidersInterface`

These are further explained in their own documentation.

## Storage

The Storage Providers pallet uses the following storage items for managing the state of the network:

### `SignUpRequests`

This storage holds the sign up requests initiated by users of StorageHub that want to offer their services as Storage Providers, both Main and Backup.

It's a map from an account ID to a tuple consisting of the Storage Provider metadata that the account used when requesting to sign up and the block number in which the request was initiated.

```rust
AccountId -> (StorageProviderMetadata, BlockWhenSignUpWasRequested)
```

### `AccountIdToMainStorageProviderId`

This storage is used to keep track of the one-to-one relationship between an account ID and a Main Storage Provider ID, which is used to choose which challenges are requested from that Storage Provider.

It's a map from an account ID to a Main Storage Provider ID, which is of the Hash type from the runtime.

```rust
AccountId -> MainStorageProviderId
```

### `MainStorageProviders`

This storage holds the metadata of each registered Main Storage Provider, including its corresponding buckets, its capacity, used data, the valid multiaddresses to connect to it, its list of value propositions and the block in which this Storage Provider last changed its capacity.

It's a map from a Main Storage Provider ID to its metadata.

```rust
MainStorageProviderId -> MainStorageProviderMetadata
```

### `Buckets`

This storage holds the metadata of each bucket that exists in each Main Storage Provider. It holds the bucket's root, the user ID that owns that bucket and the Main Storage Provider ID that holds that bucket. It is updated using the `MutateProvidersInterface`.

It's a map from a bucket ID to that bucket's metadata

```rust
BucketId -> Bucket
```

### `AccountIdToBackupStorageProviderId`

This storage is used to keep track of the one-to-one relationship between an account ID and a Backup Storage Provider ID, which is used to both choose which challenges are requested from that Storage Provider and to compare with the threshold used to allow Backup Storage Providers to offer themselves to store a new file of the system.

It's a map from an account ID to a Backup Storage Provider ID, which is of the Hash type from the runtime.

```rust
AccountId -> BackupStorageProviderId
```

### `BackupStorageProviders`

This storage holds the metadata of each registered Backup Storage Provider, which has its capacity, its used data, the valid multiaddresses to connect to it, its forest root and the block in which this Storage Provider last changed its capacity.

It's a map from a Backup Storage Provider ID to its metadata.

```rust
BackupStorageProviderId -> BackupStorageProviderMetadata
```

### `MspCount`

This storage holds the amount of Main Storage Providers that are currently registered in the system.

### `BspCount`

This storage holds the amount of Backup Storage Providers that are currently registered in the system.

### `TotalBspsCapacity`

This storage holds the sum of all the capacity that has been registered by Backup Storage Providers, which corresponds to the capacity of the whole network.

## Events

The Storage Providers pallet emits the following events:

### `MspRequestSignUpSuccess`

This event is emitted when a Main Storage Provider has requested to sign up successfully. It provides information about that Main Storage Provider's account ID, the list of valid multiaddresses that it wants to register, the total capacity that it wants to register, and its list of value propositions.

The nature of this event is to allow the caller of the extrinsic to know that the request to sign up as a Main Storage Provider was successful and that the corresponding deposit was held.

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

The nature of this event is to allow the newly registered Main Storage Provider to know that the confirmation of its request to sign up as a Main Storage Provider was successful and that from now on, the user is a Main Storage Provider and can start storing user data. It also allows users of the network to know that a new Main Storage Provider has joined it, which can be useful for them to choose which Main Storage Provider to use.

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

The nature of this event is to allow the caller of the extrinsic to know that the request to sign up as a Backup Storage Provider was successful and that the corresponding deposit was held.

```rust
BspRequestSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
}
```

### `BspSignUpSuccess`

This event is emitted when a Backup Storage Provider has confirmed its requested sign up successfully. It provides information about that Backup Storage Provider's account ID, the list of valid multiaddresses that it has registered and the total capacity that it has registered.

The nature of this event is to allow the newly registered Backup Storage Provider to know that the confirmation of its request to sign up as a Backup Storage Provider was successful and that from now on, the user is a Backup Storage Provider and can start volunteering to store user data. It also allows Main Storage Providers to know that a new Backup Storage Provider has joined the network, which can be useful for them when they need to retrieve files from the network.

```rust
BspSignUpSuccess {
    who: T::AccountId,
    multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    capacity: StorageData<T>,
}
```

### `SignUpRequestCanceled`

This event is emitted when a Storage Provider has cancelled its request to sign up successfully and the previously held deposit has been returned to it.

The nature of this event is to allow the caller of the extrinsic to know that the request to sign up as a Storage Provider was cancelled successfully and that the corresponding deposit was returned.

```rust
SignUpRequestCanceled { who: T::AccountId }
```

### `MspSignOffSuccess`

This event is emitted when a Main Storage Provider has signed off of the system successfully and the previously held deposit has been returned to it.

The nature of this event is to allow the caller of the extrinsic to know that the sign off as a Main Storage Provider was successful and that the corresponding deposit was returned. It also allows users of the network to know that this Main Storage Provider is no longer available as an option for storing their data.

```rust
MspSignOffSuccess { who: T::AccountId }
```

### `BspSignOffSuccess`

This event is emitted when a Backup Storage Provider has signed off of the system successfully and the previously held deposit has been returned to it.

The nature of this event is to allow the caller of the extrinsic to know that the sign off as a Backup Storage Provider was successful and that the corresponding deposit was returned. It also allows Main Storage Providers to know that this Backup Storage Provider is no longer available as an option for retrieving data.

```rust
BspSignOffSuccess { who: T::AccountId }
```

### `CapacityChanged`

This event is emitted when a Storage Provider has successfully changed its registered capacity on the network. It holds the information about the account ID of that Storage Provider, its previous capacity, the new registered capacity and the next block after which the timelock expires and it is able to change its capacity again.

The nature of this event is to allow the caller of the extrinsic to know that the change of capacity was successful and that the difference in deposit was held or returned. It also allows users of the network to know that this Storage Provider has changed its capacity, which can be useful for them to choose which Storage Provider to use.

```rust
CapacityChanged {
    who: T::AccountId,
    old_capacity: StorageData<T>,
    new_capacity: StorageData<T>,
    next_block_when_change_allowed: BlockNumberFor<T>,
}
```

## Errors

The Storage Providers pallet uses the following error types:

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

## Slashing Protocol

Storage Providers who fail to submit a proof by the last challenge tick will be slashed, predetermined by the challenge period defined in the proofs-dealer pallet.

Slashing is an asynchronous process, therefore it is possible for a Storage Provider to have failed more than one challenge before being slashed. To avoid all possibility for a Storage Provider to not be slashed for the number of failed proof submissions, the runtime will accrue the number of failed challenges for each Storage Provider. Slashing a Storage Provider will take into account the total number of failed challenges and multiply it by a configurable slash factor.

### Manual and Automatic Slashing

The `slash` extrinsic can be called by any account to manually slash a Storage Provider and only requires the Storage Provider ID of a Storage Provider to be slashed, be it either an MSP or a BSP.

An automated slashing mechanism is implemented in an off-chain worker process to be executed by collators which efficiently slashes many Storage Providers.

### Grace Period and Insolvency

Since the Storage Provider's stake determines their total storage capacity, it is entirely possible that the amount of data currently stored would be above their total storage capacity after slashing. StorageHub grants a predetermined configurable grace period for Storage Providers to top-up their stake to have their total capacity equal to or greater than the amount of data they currently store. If the grace period has been reached, they are considered to be insolvent and cease to be a Storage Provider.

The grace period is based on the total stake/capacity of the Storage Provider. In essence, the more stake a Storage Provider has, the longer the grace period.
This is to avoid a high stake Storage Provider from being removed from the network prematurely.

The runtime will automatically process any expired grace periods within the `on_poll` hook to ensure that the redundancy process is initiated as soon as possible. For every insolvent Storage Provider, an event will be emitted to notify the network and also mark the Storage Provider as insolvent, rendering them unable to operate as a Storage Provider. Finally all the of the Storage Provider's stake will be slashed and transfered to the treasury.

### Ensuring Data Redundancy

> [!IMPORTANT]
> The runtime cannot ensure that all the data stored from an insolvent storage provider would be recovered. It is up to users and storage providers to ensure data redundancy since the runtime has no knowledge of file keys stored by whom.

In the event when a BSP would become insolvent, the entire network of BSPs are responsible to regain data redundancy for the data they lost.

To accomplish this, an off-chain indexer is required to discover the file keys which were stored by that insolvent Storage Provider. This is necessary since the runtime does not hold any file key in data in storage.

Any account can call the `add_redundancy` extrinsic which requires a proof of inclusion of a given file key and the number of required BSPs needed to fulfill this request. The root is checked to be a current Bucket or BSP’s forest root to ensure that the file key does indeed exist as part of a Storage Provider's forest.

This creates a traditional storage request with the specified amount of BSPs required. The caller of the extrinsic can optionally pass a list of data servers for the file key, which then will be marked in the storage request for the volunteers to request the data from. The caller is be able to obtain this information from the off-chain indexer.

If the file was originally stored by an MSP, it is up to the user of the lost file or files within a bucket to execute the `transfer_file` or `transfer_bucket` extrinsics exposed by the file system pallet to move the data to a new MSP.

#### Incentives and Storage Cleanup

For every file key submitted for redundancy which was stored by an insolvent Storage Provider, the caller will be rewarded with a configurable amount of tokens which must be less than the slash factor to prevent abuse. The runtime will accrue the file size of each file key submitted for redundancy for the given insolvent Storage Provider. Once the total accrued file size reaches the total data size stored by the insolvent Storage Provider, the Storage Provider is deleted from the runtime.

This process ensures that total redundancy is regained before the insolvent Storage Provider is removed from the network.
