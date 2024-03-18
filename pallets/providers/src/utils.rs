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
impl<T: pallet::Config> StorageProvidersInterface<T> for pallet::Pallet<T> {
    fn get_sp(who: T::AccountId) -> Option<BackupStorageProvider<T>> {
        if let Some(m) = Msps::<T>::get(&who) {
            Some(m.into())
        } else if let Some(b) = Bsps::<T>::get(&who) {
            Some(b)
        } else {
            None
        }
    }

    fn is_sp(who: T::AccountId) -> bool {
        Msps::<T>::contains_key(&who) || Bsps::<T>::contains_key(&who)
    }

    fn total_sps() -> T::SpCount {
        // TODO: Add checks
        MspCount::<T>::get() + BspCount::<T>::get()
    }

    fn get_stake(_who: BackupStorageProvider<T>) -> BalanceOf<T> {
        T::SpMinDeposit::get()
        // TODO: Implement this
        //T::SpMinDeposit::get() + (T::DepositPerData::get() * who.total_data.into())
    }

    fn change_data_used(who: &T::AccountId, data_change: T::StorageData) -> DispatchResult {
        // TODO: refine this logic, add checks
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

    // MSP specific functions:

    fn add_or_change_root_msp(
        who: &T::AccountId,
        new_root: MerklePatriciaRoot<T>,
        user_id: T::UserId,
        bucket_id: T::BucketId,
    ) -> DispatchResult {
        if let Some(m) = Msps::<T>::get(&who) {
            MspToUserToBucketToRoot::<T>::insert((&m, &user_id, &bucket_id), &new_root);
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn remove_root_msp(
        who: &<T>::AccountId,
        user_id: <T as Config>::UserId,
        bucket_id: <T as Config>::BucketId,
    ) -> DispatchResult {
        if let Some(m) = Msps::<T>::get(&who) {
            MspToUserToBucketToRoot::<T>::remove((&m, &user_id, &bucket_id));
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn is_valid_root_for_msp(
        who: &<T>::AccountId,
        root: MerklePatriciaRoot<T>,
        user_id: <T as Config>::UserId,
        bucket_id: <T as Config>::BucketId,
    ) -> bool {
        if let Some(m) = Msps::<T>::get(&who) {
            if let Some(r) = MspToUserToBucketToRoot::<T>::get((&m, &user_id, &bucket_id)) {
                r == root
            } else {
                false
            }
        } else {
            false
        }
    }

    // BSP specific functions:

    fn change_root_bsp(who: &<T>::AccountId, new_root: MerklePatriciaRoot<T>) -> DispatchResult {
        if let Some(b) = Bsps::<T>::get(&who) {
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

    fn remove_root_bsp(who: &<T>::AccountId) -> DispatchResult {
        Bsps::<T>::remove(&who);
        Ok(())
    }

    fn is_valid_root_for_bsp(who: &<T>::AccountId, root: MerklePatriciaRoot<T>) -> bool {
        if let Some(b) = Bsps::<T>::get(&who) {
            b.root == root
        } else {
            false
        }
    }
}
