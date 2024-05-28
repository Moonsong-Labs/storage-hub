use codec::Encode;
use frame_support::ensure;
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;
use storage_hub_traits::ReadProvidersInterface;

use crate::types::ReadAccessRegex;
use crate::{
    pallet,
    types::{AccountIdLookupSourceOf, AccountIdLookupTargetOf, BucketIdFor, ItemMetadata},
    Error, Pallet,
};

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    /// Share access by issuing an item for a given bucket to the `recipient` account.
    pub(crate) fn do_share_access(
        issuer: &T::AccountId,
        recipient: AccountIdLookupSourceOf<T>,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
        read_access_regex: Option<ReadAccessRegex<T>>,
    ) -> Result<AccountIdLookupTargetOf<T>, DispatchError> {
        // Convert the lookup source to a target account.
        let recipient_account = T::Lookup::lookup(recipient.clone())?;

        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_collection_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::BucketIsNotPrivate)?;

        // Check if the issuer is the owner of the bucket.
        // This is a redundant check but primarily added for ergonomics.
        // Transfering ownership of a collection is not exposed to the user, therefore the bucket owner is implicitly the collection owner.
        ensure!(
            T::Providers::is_bucket_owner(issuer, &bucket)?,
            Error::<T>::NotBucketOwner
        );

        let origin = Self::sign(issuer);

        // Issue an item to the account.
        pallet_nfts::Pallet::<T>::mint(origin.clone(), collection_id, item_id, recipient, None)?;

        // Create the metadata for the item.
        let metadata = ItemMetadata::<T>::new(read_access_regex);

        // Set the read access regex for the item.
        pallet_nfts::Pallet::<T>::set_metadata(
            origin,
            collection_id,
            item_id,
            metadata.encode().try_into().unwrap(),
        )?;

        Ok(recipient_account)
    }

    /// Update the read access regex for an item.
    pub(crate) fn do_update_read_access(
        account: &T::AccountId,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
        read_access_regex: Option<ReadAccessRegex<T>>,
    ) -> Result<(), DispatchError> {
        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_collection_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::BucketIsNotPrivate)?;

        // Check if the issuer is the owner of the bucket.
        // This is a redundant check but primarily added for ergonomics.
        // Transfering ownership of a collection is not exposed to the user and therefore the bucket owner is implicitly the collection owner.
        ensure!(
            T::Providers::is_bucket_owner(account, &bucket)?,
            Error::<T>::NotBucketOwner
        );

        // We do not add any additional redundant checks already covered by the `set_metadata` function from the `pallet-nfts` pallet.
        // For example, we do not check if the item exists.

        let metadata = ItemMetadata::<T>::new(read_access_regex);

        // Set the read access regex for the item.
        pallet_nfts::Pallet::<T>::set_metadata(
            Self::sign(account),
            collection_id,
            item_id,
            metadata.encode().try_into().unwrap(),
        )?;

        Ok(())
    }

    /// Helper function to create a signed `RuntimeOrigin(RawOrigin)`.
    fn sign(account: &T::AccountId) -> OriginFor<T> {
        OriginFor::<T>::from(RawOrigin::Signed(account.clone()))
    }
}
