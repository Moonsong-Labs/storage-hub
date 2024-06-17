use sp_core::H256;

pub fn to_h256<T: AsRef<[u8]>>(data: T) -> H256 {
    let mut res = H256::zero();
    res.assign_from_slice(data.as_ref());
    res
}
