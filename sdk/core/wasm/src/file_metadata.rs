use wasm_bindgen::prelude::*;

use parity_scale_codec::Encode;
use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
use shp_file_metadata::{FileMetadata as RustFileMetadata, Fingerprint};
use sp_core::hashing::blake2_256;

// ────────────────────────────────────────────────────────────────────────────
// WASM‐exposed wrapper for FileMetadata
// ────────────────────────────────────────────────────────────────────────────
#[wasm_bindgen]
pub struct FileMetadata {
    inner: RustFileMetadata<{ H_LENGTH }, { FILE_CHUNK_SIZE }, { FILE_SIZE_TO_CHALLENGES }>,
}

#[wasm_bindgen]
impl FileMetadata {
    /// Constructs a new `FileMetadata`.
    /// * `owner`, `bucket_id`, `fingerprint` – 32-byte arrays (passed as slices)
    /// * `location` – arbitrary byte string (file path)
    /// * `size` – file size in bytes
    #[wasm_bindgen(constructor)]
    pub fn new(
        owner: &[u8],
        bucket_id: &[u8],
        location: &[u8],
        size: u64,
        fingerprint: &[u8],
    ) -> Result<FileMetadata, JsValue> {
        // Basic validation to return early, preventing panics deeper inside.
        if owner.len() != H_LENGTH
            || bucket_id.len() != H_LENGTH
            || fingerprint.len() != H_LENGTH
            || location.is_empty()
            || size == 0
        {
            return Err(JsValue::from_str("Invalid FileMetadata parameters"));
        }

        let inner = RustFileMetadata::new(
            owner.to_vec(),
            bucket_id.to_vec(),
            location.to_vec(),
            size,
            Fingerprint::from(<[u8; H_LENGTH]>::try_from(fingerprint).unwrap()),
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create FileMetadata: {:?}", e)))?;

        Ok(FileMetadata { inner })
    }

    /// Returns the FileKey (blake2_256 hash of SCALE-encoded metadata) as a
    /// 32-byte `Uint8Array`.
    #[wasm_bindgen(js_name = getFileKey)]
    pub fn get_file_key(&self) -> Vec<u8> {
        blake2_256(&self.inner.encode()).to_vec()
    }
}
