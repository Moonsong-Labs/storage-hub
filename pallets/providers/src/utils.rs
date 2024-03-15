use frame_support::pallet_prelude::DispatchResult;

use crate::*;

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn do_msp_sign_up(who: &T::AccountId, msp_info: MainStorageProvider<T>) -> DispatchResult {
        // todo!()
        <Msps<T>>::insert(&who, msp_info);
        Ok(())
    }

    pub fn do_bsp_sign_up(
        who: &T::AccountId,
        bsp_info: BackupStorageProvider<T>,
    ) -> DispatchResult {
        todo!()
        /* <Bsps<T>>::insert(
            &who,
            bsp_info,
        ); */
    }

    pub fn do_msp_sign_off(who: &T::AccountId) -> DispatchResult {
        todo!()
    }

    pub fn do_bsp_sign_off(who: &T::AccountId) -> DispatchResult {
        todo!()
    }
}

/// Implement the StorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> StorageProvidersInterface for pallet::Pallet<T> {
    /// Trait types
    type AccountId = T::AccountId;
    type Balance = T::NativeBalance;
    type StorageData = T::StorageData;
    /// Since the ones using the interface do not care about the value proposition, we can use the backup storage provider type.
    type StorageProvider = BackupStorageProvider<T>;
    type UserCount = T::UserCount;

    fn get_sp(who: Self::AccountId) -> Option<Self::StorageProvider> {
        if let Some(m) = Msps::<T>::get(who) {
            Some(m.into())
        } else if let Some(b) = Bsps::<T>::get(who) {
            Some(b)
        } else {
            None
        }
    }

    fn is_sp(who: Self::AccountId) -> bool {
        Msps::<T>::contains_key(who) || Bsps::<T>::contains_key(who)
    }

    fn total_sps() -> Self::UserCount {
        UserCount::<T>::get()
    }

    fn get_stake(who: Self::StorageProvider) -> Self::Balance {}
}
