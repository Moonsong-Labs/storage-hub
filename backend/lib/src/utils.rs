pub mod serde {
    use alloy_core::primitives::Address;
    use serde::Serializer;

    pub fn hex_string<T: AsRef<[u8]>, S: Serializer>(item: &T, ser: S) -> Result<S::Ok, S::Error> {
        let s = hex::encode(item.as_ref());
        ser.serialize_str(&s)
    }

    pub fn checksummed_address<S: Serializer>(
        address: &Address,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let address = address.to_checksum(None);
        ser.serialize_str(&address)
    }
}
