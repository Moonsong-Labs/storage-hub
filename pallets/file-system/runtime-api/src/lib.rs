#![cfg_attr(not(feature = "std"), no_std)]

use scale_info::prelude::vec::Vec;

use shp_file_key_verifier::types::ChunkId;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait FileSystemApi<ProviderId, FileKey, BlockNumber>
    where
        ProviderId: codec::Codec,
        FileKey: codec::Codec,
        BlockNumber: codec::Codec,
    {
        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: ProviderId, file_key: FileKey) -> Vec<ChunkId>;
    }
}
