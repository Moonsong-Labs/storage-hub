use wasm_bindgen::prelude::*;

use parity_scale_codec::Encode;
use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
use shp_file_metadata::FileMetadataError;
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
        let fp_arr: [u8; H_LENGTH] = fingerprint
            .try_into()
            .map_err(|_| JsValue::from_str("fingerprint must be 32 bytes"))?;

        let inner = RustFileMetadata::new(
            owner.to_vec(),
            bucket_id.to_vec(),
            location.to_vec(),
            size,
            Fingerprint::from(fp_arr),
        )
        .map_err(|e| {
            JsValue::from_str(match e {
                FileMetadataError::InvalidOwner => "owner must not be an empty array",
                FileMetadataError::InvalidBucketId => "invalid bucket_id (32-byte hash expected)",
                FileMetadataError::InvalidLocation => "location must not be empty",
                FileMetadataError::InvalidFileSize => "size must be greater than 0",
                FileMetadataError::InvalidFingerprint => {
                    "invalid fingerprint (32-byte hash expected)"
                }
            })
        })?;

        Ok(FileMetadata { inner })
    }

    /// Returns the FileKey (blake2_256 hash of SCALE-encoded metadata) as a
    /// 32-byte `Uint8Array`.
    #[wasm_bindgen(js_name = getFileKey)]
    pub fn get_file_key(&self) -> Vec<u8> {
        blake2_256(&self.inner.encode()).to_vec()
    }
}
