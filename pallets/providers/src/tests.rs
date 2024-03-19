use crate::mock::*;
use crate::types::{ValuePropId, ValueProposition};
use frame_support::assert_ok;
use sp_runtime::BoundedVec;

#[test]
fn it_does_something() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(StorageProviders::msp_sign_up(
            RuntimeOrigin::signed(1),
            42,
            BoundedVec::new(),
            ValueProposition {
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new()
            }
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(StorageProviders::get_total_capacity(&1).unwrap(), 42);
    });
}
