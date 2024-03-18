use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::Get;

use crate::*;

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn do_msp_sign_up(who: &T::AccountId, msp_info: &MainStorageProvider<T>) -> DispatchResult {
        // todo!()
        <Msps<T>>::insert(&who, msp_info);
        Ok(())
    }

    pub fn do_bsp_sign_up(
        who: &T::AccountId,
        bsp_info: BackupStorageProvider<T>,
    ) -> DispatchResult {
        // todo!()
        <Bsps<T>>::insert(&who, bsp_info);
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
            multiaddress: msp.multiaddress,
            root: MerklePatriciaRoot::<T>::default(),
        }
    }
}

/// Implement the StorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> StorageProvidersInterface for pallet::Pallet<T> {
    /// Trait types
    type AccountId = T::AccountId;
    type Balance = T::NativeBalance;
    type StorageData = T::StorageData;
    /// Since the ones using the interface do not care about the value proposition, we can use the Backup Storage Provider type.
    type StorageProvider = BackupStorageProvider<T>;
    type SpCount = T::SpCount;
    type MerklePatriciaRoot = T::MerklePatriciaRoot;
    type UserId = T::UserId;
    type BucketId = T::BucketId;

    fn get_sp(who: Self::AccountId) -> Option<Self::StorageProvider> {
        if let Some(m) = Msps::<T>::get(&who) {
            Some(m.into())
        } else if let Some(b) = Bsps::<T>::get(&who) {
            Some(b)
        } else {
            None
        }
    }

    fn is_sp(who: Self::AccountId) -> bool {
        Msps::<T>::contains_key(&who) || Bsps::<T>::contains_key(&who)
    }

    fn total_sps() -> Self::SpCount {
        SpCount::<T>::get()
    }

    fn get_stake(_who: Self::StorageProvider) -> BalanceOf<T> {
        T::SpMinDeposit::get()
        // TODO: Implement this
        //T::SpMinDeposit::get() + (T::DepositPerData::get() * who.total_data.into())
    }

    fn change_data_used(who: &Self::AccountId, data_change: Self::StorageData) -> DispatchResult {
        if let Some(m) = Msps::<T>::get(&who) {
            Msps::<T>::insert(
                who,
                MainStorageProvider {
                    data_used: m.data_used + data_change,
                    ..m
                },
            );
        } else if let Some(b) = Bsps::<T>::get(&who) {
            Bsps::<T>::insert(
                who,
                BackupStorageProvider {
                    data_used: b.data_used + data_change,
                    ..b
                },
            );
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    // todo!("ASK IF THIS SHOULDN'T BE TWO DIFFERENT SET OF FUNCTIONS (for MSPs and BSPs)")
    fn add_or_change_root(
        who: &Self::AccountId,
        new_root: Self::MerklePatriciaRoot,
        user_id: Option<Self::UserId>,
        bucket_id: Option<Self::BucketId>,
    ) -> DispatchResult {
        if let Some(m) = Msps::<T>::get(&who) {
            if let Some(u) = user_id {
                if let Some(b) = bucket_id {
                    MspToUserToBucketToRoot::<T>::insert((&m, &u, &b), &new_root);
                } else {
                    return Err(Error::<T>::NoBucketId.into());
                }
            } else {
                return Err(Error::<T>::NoUserId.into());
            }
        } else if let Some(b) = Bsps::<T>::get(&who) {
            Bsps::<T>::insert(
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

    fn remove_root(
        who: &Self::AccountId,
        user_id: Option<Self::UserId>,
        bucket_id: Option<Self::BucketId>,
    ) -> DispatchResult {
        if let Some(m) = Msps::<T>::get(&who) {
            if let Some(u) = user_id {
                if let Some(b) = bucket_id {
                    MspToUserToBucketToRoot::<T>::remove((&m, &u, &b));
                } else {
                    return Err(Error::<T>::NoBucketId.into());
                }
            } else {
                return Err(Error::<T>::NoUserId.into());
            }
        } else if let Some(_b) = Bsps::<T>::get(&who) {
            Bsps::<T>::remove(who);
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    // todo!("Ask Facu if this works for him, as I'm not sure if he has access to the user IDs and bucket IDs")
    // The alternative would be to iterate through all existing users and buckets, and check if the root is valid for any of them.
    // That would be awful...
    fn is_valid_root_for_sp(
        who: &Self::AccountId,
        root: Self::MerklePatriciaRoot,
        user_id: Option<Self::UserId>,
        bucket_id: Option<Self::BucketId>,
    ) -> bool {
        if let Some(b) = Bsps::<T>::get(&who) {
            b.root == root
        } else if let Some(m) = Msps::<T>::get(&who) {
            match (user_id, bucket_id) {
                (Some(u), Some(b)) => {
                    if let Some(r) = MspToUserToBucketToRoot::<T>::get((&m, &u, &b)) {
                        r == root
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    }
}
