pub const SAVE_FILE_TO_DISK: &str = "storagehubclient_saveFileToDisk";
pub const FILE_KEY_EXPECTED: &str = "storagehubclient_isFileKeyExpected";
pub const IS_FILE_IN_FILE_STORAGE: &str = "storagehubclient_isFileInFileStorage";
// TODO: Remove this constant once legacy upload is deprecated
pub const RECEIVE_FILE_CHUNKS: &str = "storagehubclient_receiveBackendFileChunks";
pub const PROVIDER_ID: &str = "storagehubclient_getProviderId";
pub const VALUE_PROPS: &str = "storagehubclient_getValuePropositions";
pub const PEER_IDS: &str = "system_localListenAddresses";

pub const API_CALL: &str = "state_call";
pub const STATE_QUERY: &str = "state_getStorage";

// Substrate standard RPCs used for node health checks
pub const FINALIZED_HEAD: &str = "chain_getFinalizedHead";
pub const GET_HEADER: &str = "chain_getHeader";
pub const ACCOUNT_NEXT_INDEX: &str = "system_accountNextIndex";
