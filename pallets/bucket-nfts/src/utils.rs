use codec::Encode;
use frame_support::ensure;
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use sp_runtime::traits::StaticLookup;
use sp_runtime::{BoundedVec, DispatchError};
use storage_hub_traits::ReadProvidersInterface;

use crate::{
    pallet,
    types::{AccountIdLookupSourceOf, AccountIdLookupTargetOf, BucketIdFor, ItemMetadata},
    Error, Pallet,
};

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    /// Share access by issuing an item to the account for a given bucket.
    pub(crate) fn do_share_access(
        issuer: &T::AccountId,
        recipient: AccountIdLookupSourceOf<T>,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
        read_access_regex: BoundedVec<u8, T::StringLimit>,
    ) -> Result<AccountIdLookupTargetOf<T>, DispatchError> {
        // Convert the lookup source to a target account.
        let recipient_account = T::Lookup::lookup(recipient.clone())?;

        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_collection_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::BucketIsNotPrivate)?;

        // Check if the issuer is the owner of the bucket.
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
        read_access_regex: BoundedVec<u8, T::StringLimit>,
    ) -> Result<(), DispatchError> {
        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_collection_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::BucketIsNotPrivate)?;

        // Check if the issuer is the owner of the bucket.
        ensure!(
            T::Providers::is_bucket_owner(account, &bucket)?,
            Error::<T>::NotBucketOwner
        );

        // Check if the item exists.
        pallet_nfts::Item::<T>::get(collection_id, item_id).ok_or(Error::<T>::ItemNotFound)?;

        // Set the read access regex for the item.
        pallet_nfts::Pallet::<T>::set_metadata(
            Self::sign(account),
            collection_id,
            item_id,
            read_access_regex,
        )?;

        Ok(())
    }

    /// Burn an item from a collection.
    ///
    /// Only the owner of the item can burn it. If the owner of the bucket wishes to remove access from an account, they should use the `update_read_access` function.
    pub(crate) fn do_burn(
        account: &T::AccountId,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
    ) -> Result<(), DispatchError> {
        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_collection_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::BucketIsNotPrivate)?;

        // Check if the item exists.
        pallet_nfts::Item::<T>::get(collection_id, item_id).ok_or(Error::<T>::ItemNotFound)?;

        // Burn the item.
        pallet_nfts::Pallet::<T>::burn(Self::sign(account), collection_id, item_id)?;

        Ok(())
    }

    /// Helper function to create a signed `RawOrigin`.
    fn sign(account: &T::AccountId) -> OriginFor<T> {
        OriginFor::<T>::from(RawOrigin::Signed(account.clone()))
    }
}
