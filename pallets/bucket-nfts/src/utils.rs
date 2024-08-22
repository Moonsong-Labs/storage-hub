use codec::Encode;
use frame_support::ensure;
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use shp_traits::ReadBucketsInterface;
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;

use crate::types::ReadAccessRegex;
use crate::{
    pallet,
    types::{
        AccountIdLookupSourceOf, AccountIdLookupTargetOf, BucketIdFor, CollectionIdFor,
        ItemMetadata,
    },
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

        // Check if the bucket is private.
        ensure!(
            T::Buckets::is_bucket_private(&bucket)?,
            Error::<T>::BucketIsNotPrivate
        );

        // Get the collection ID of the bucket.
        // It is possible that the collection ID does not exist since users
        // can delete collections by calling the nfts pallet directly. Users
        // can call `create_and_associate_collection` from the file system pallet to fix this.
        let collection_id = T::Buckets::get_read_access_group_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::NoCorrespondingCollection)?;

        let origin = Self::sign(issuer);

        // Issue an item to the account.
        pallet_nfts::Pallet::<T>::mint(origin.clone(), collection_id, item_id, recipient, None)?;

        // Create the metadata for the item.
        let metadata = ItemMetadata::<T>::new(read_access_regex);

        let encoded_metadata = metadata
            .encode()
            .try_into()
            .map_err(|_| Error::<T>::ConvertBytesToBoundedVec)?;

        // Set the read access regex for the item.
        pallet_nfts::Pallet::<T>::set_metadata(origin, collection_id, item_id, encoded_metadata)?;

        Ok(recipient_account)
    }

    /// Update the read access regex for an item.
    pub(crate) fn do_update_read_access(
        account: &T::AccountId,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
        read_access_regex: Option<ReadAccessRegex<T>>,
    ) -> Result<(), DispatchError> {
        // Check if the bucket is private.
        ensure!(
            T::Buckets::is_bucket_private(&bucket)?,
            Error::<T>::BucketIsNotPrivate
        );

        // Get the collection ID of the bucket.
        // This should never fail because the file system pallet ensures that collections are created whenever a bucket is created with private access
        // or when a bucket is made private after being public.
        let collection_id = T::Buckets::get_read_access_group_id_of_bucket(&bucket)?
            .ok_or(Error::<T>::NoCorrespondingCollection)?;

        // We do not add any additional redundant checks already covered by the `set_metadata` function from the `pallet-nfts` pallet.
        // For example, we do not check if the item exists or if the account is the owner of the collection.

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

impl<T: pallet::Config> shp_traits::InspectCollections for Pallet<T> {
    type CollectionId = CollectionIdFor<T>;

    fn collection_exists(collection_id: &Self::CollectionId) -> bool {
        pallet_nfts::Collection::<T>::contains_key(collection_id)
    }
}
