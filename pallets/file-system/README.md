# Pallet File System Pallet

## Design

### File Deletion

As a reminder, files stored by providers, be it BSPs or MSPs, can only be removed from a provider's _forest_ once they provide a proof of inclusion. The runtime is responsible for verifying the proof and removing the leaf (file) from the _forest_.

User's who wish to delete a file will initially call the `delete_file` extrinsic. This extrinsic optionally accepts a proof of inclusion of the file in a bucket's _forest_. MSPs will expose an RPC endpoint to allow users to request a proof of inclusion of a file in their _forest_.

If the proof is provided, the runtime will verify the proof and queue a priority challenge in the proofs-dealer pallet paired with `TrieRemoveMutation`, forcing all providers to provide a proof of (non-)inclusion of the challenged file. The runtime will automatically remove the file from the _forest_ for all providers who submitted a proof of inclusion.

If the proof is not provided, a pending file deletion request is created with a defined constant expiration time. The MSP who is supposed to be storing the file, will have until the expiration time to provide a proof of (non-)inclusion of the file in their _forest_. If it is a proof of non-inclusion, this means the file does not exist, and the pending file deletion request will be removed. This prevents all the providers from responding to a priority challenge for a file that does not exist. If it is a proof of inclusion, a priority challenge will be queued in the proofs-dealer pallet, following the same process described above.

Once a pending file deletion request reaches its expiration time and it has not been responded to, a priority challenge will be queued, following the same process described above.
