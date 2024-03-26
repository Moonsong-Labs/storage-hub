use crate::types::{Bucket, MainStorageProvider};
use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::Get;
use storage_hub_traits::{MutateProvidersInterface, ReadProvidersInterface};

use crate::*;

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn do_msp_sign_up(
        _who: &T::AccountId,
        _msp_info: &MainStorageProvider<T>,
    ) -> DispatchResult {
        // todo!()
        // let msp_id =
        //    AccountIdToMainStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        // <MainStorageProviders<T>>::insert(&msp_id, msp_info);
        Ok(())
    }

    pub fn do_bsp_sign_up(
        who: &T::AccountId,
        bsp_info: BackupStorageProvider<T>,
    ) -> DispatchResult {
        // todo!()
        let bsp_id =
            AccountIdToBackupStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        <BackupStorageProviders<T>>::insert(&bsp_id, bsp_info);
        Ok(())
    }

    pub fn do_msp_sign_off(_who: &T::AccountId) -> DispatchResult {
        todo!()
    }

    pub fn do_bsp_sign_off(_who: &T::AccountId) -> DispatchResult {
        todo!()
    }
}

impl<T: Config> From<MainStorageProvider<T>> for BackupStorageProvider<T> {
    fn from(msp: MainStorageProvider<T>) -> Self {
        BackupStorageProvider {
            total_data: msp.total_data,
            data_used: msp.data_used,
            multiaddresses: msp.multiaddresses,
            root: MerklePatriciaRoot::<T>::default(),
        }
    }
}

/// Implement the StorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> MutateProvidersInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type Provider = T::HashId;
    type StorageData = T::StorageData;
    type BucketId = T::HashId;
    type MerklePatriciaRoot = T::MerklePatriciaRoot;

    fn change_data_used(who: &T::AccountId, data_change: T::StorageData) -> DispatchResult {
        // TODO: refine this logic, add checks
        if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            let mut msp =
                MainStorageProviders::<T>::get(&msp_id).ok_or(Error::<T>::NotRegistered)?;
            msp.data_used += data_change;
            MainStorageProviders::<T>::insert(&msp_id, msp);
        } else if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            let mut bsp =
                BackupStorageProviders::<T>::get(&bsp_id).ok_or(Error::<T>::NotRegistered)?;
            bsp.data_used += data_change;
            BackupStorageProviders::<T>::insert(&bsp_id, bsp);
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    // Bucket specific functions:
    fn add_bucket(
        msp_id: MainStorageProviderId<T>,
        user_id: T::AccountId,
        bucket_id: BucketId<T>,
        bucket_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult {
        // TODO: Check that the bucket does not exist yet
        // TODO: Get BucketId by hashing Bucket with salt, add it to the MSP vector of buckets
        let bucket = Bucket {
            root: bucket_root,
            user_id,
            msp_id,
        };
        Buckets::<T>::insert(&bucket_id, &bucket);
        Ok(())
    }

    fn change_root_bucket(
        bucket_id: BucketId<T>,
        new_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult {
        if let Some(bucket) = Buckets::<T>::get(&bucket_id) {
            Buckets::<T>::insert(
                &bucket_id,
                Bucket {
                    root: new_root,
                    ..bucket
                },
            );
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn remove_root_bucket(bucket_id: BucketId<T>) -> DispatchResult {
        Buckets::<T>::remove(&bucket_id);
        Ok(())
    }

    // BSP specific functions:
    fn change_root_bsp(
        who: BackupStorageProviderId<T>,
        new_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult {
        if let Some(b) = BackupStorageProviders::<T>::get(&who) {
            BackupStorageProviders::<T>::insert(
                who,
                BackupStorageProvider {
                    root: new_root,
                    ..b
                },
            );
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn remove_root_bsp(who: &<T>::AccountId) -> DispatchResult {
        let bsp_id =
            AccountIdToBackupStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        BackupStorageProviders::<T>::remove(&bsp_id);
        AccountIdToBackupStorageProviderId::<T>::remove(&who);
        Ok(())
    }
}

impl<T: pallet::Config> ReadProvidersInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type Provider = HashId<T>;
    type Balance = T::NativeBalance;
    type MerkleHash = MerklePatriciaRoot<T>;

    // TODO: Refine, add checks and tests for all the logic in this implementation

    fn is_provider(who: Self::Provider) -> bool {
        BackupStorageProviders::<T>::contains_key(&who)
            || MainStorageProviders::<T>::contains_key(&who)
            || Buckets::<T>::contains_key(&who)
    }

    fn get_provider(who: Self::AccountId) -> Option<Self::Provider> {
        if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(&who) {
            Some(bsp_id)
        } else if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(&who) {
            Some(msp_id)
        } else {
            None
        }
    }

    fn get_root(who: Self::Provider) -> Option<Self::MerkleHash> {
        if let Some(bucket) = Buckets::<T>::get(&who) {
            Some(bucket.root)
        } else if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.root)
        } else {
            None
        }
    }

    fn get_stake(who: Self::Provider) -> Option<BalanceOf<T>> {
        // TODO: This is not the stake, this logic will be done later down the line
        if let Some(bucket) = Buckets::<T>::get(&who) {
            let _related_msp = MainStorageProviders::<T>::get(bucket.msp_id);
            Some(T::SpMinDeposit::get())
        } else if let Some(_bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(T::SpMinDeposit::get())
        } else {
            None
        }
    }
}
