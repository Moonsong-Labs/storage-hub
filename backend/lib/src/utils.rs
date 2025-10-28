pub mod serde {
    use serde::Serializer;

    pub fn hex_string<T: AsRef<[u8]>, S: Serializer>(item: &T, ser: S) -> Result<S::Ok, S::Error> {
        let s = hex::encode(item.as_ref());
        ser.serialize_str(&s)
    }
}
