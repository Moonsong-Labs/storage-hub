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
