use codec::Encode;
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
    /// Share access by issuing an NFT to the account for a given bucket.
    pub(crate) fn do_share_access(
        issuer: &T::AccountId,
        recipient: AccountIdLookupSourceOf<T>,
        bucket: BucketIdFor<T>,
        item_id: T::ItemId,
        read_access_regex: BoundedVec<u8, T::StringLimit>,
    ) -> Result<AccountIdLookupTargetOf<T>, DispatchError> {
        // Convert the lookup source to a target account.
        let recipient_account = T::Lookup::lookup(recipient.clone())?;

        // Ensure the account is a provider.
        let maybe_collection_id = T::Providers::get_collection_id_of_bucket(&bucket);

        // Collections only exist for private buckets.
        let collection_id = maybe_collection_id.ok_or(Error::<T>::BucketIsNotPrivate)?;

        let origin_issuer = OriginFor::<T>::from(RawOrigin::Signed(issuer.clone()));

        // Issue an NFT to the account.
        pallet_nfts::Pallet::<T>::mint(
            origin_issuer.clone(),
            collection_id,
            item_id,
            recipient,
            None,
        )?;

        // Create the metadata for the item.
        let metadata = ItemMetadata::<T>::new(read_access_regex);

        // Set the read access regex for the item.
        pallet_nfts::Pallet::<T>::set_metadata(
            origin_issuer,
            collection_id,
            item_id,
            metadata.encode().try_into().unwrap(),
        )?;

        Ok(recipient_account)
    }
}
