use crate::mock::*;

#[test]
fn set_babe_randomness_is_mandatory() {
    use frame_support::dispatch::{DispatchClass, GetDispatchInfo};

    let info = crate::Call::<Test>::set_babe_randomness {}.get_dispatch_info();
    assert_eq!(info.class, DispatchClass::Mandatory);
}
