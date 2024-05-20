# Payment Streams pallet

A pallet to create payment streams between Storage Providers and users, where other pallets through the provided interfaces can create new payment streams with a given rate of tokens per block, update those rates and delete a payment stream altogether.
The pallet aims to be configurable but not usage agnostic as it is tightly related to the Storage Providers identity from the `storage-providers` pallet that can be found in the StorageHub runtime.
