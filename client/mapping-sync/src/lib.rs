use sp_runtime::traits::Block as BlockT;

pub type StorageHubBlockNotificationSinks<T> =
    parking_lot::Mutex<Vec<sc_utils::mpsc::TracingUnboundedSender<T>>>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StorageHubBlockNotification<Block: BlockT> {
    pub is_new_best: bool,
    pub hash: Block::Hash,
}
