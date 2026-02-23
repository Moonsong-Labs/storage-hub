use codec::{Decode, DecodeWithMemTracking, Encode, FullCodec};
use core::marker::PhantomData;
use frame_support::traits::{Len, PalletError};
use frame_support::weights::{RuntimeDbWeight, Weight};
use frame_support::{BoundedVec, StorageValue};
use scale_info::TypeInfo;
use sp_core::Get;

#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
pub enum QueueError {
    IndexOutOfRange,
}

impl PalletError for QueueError {
    const MAX_ENCODED_SIZE: usize = 1;
}

pub struct BoundedQueue<W, ELEMENT, SIZE, Q, P> {
    storage_queue: PhantomData<(W, ELEMENT, SIZE, Q, P)>,
}

impl<W, ELEMENT, SIZE, Q, P> BoundedQueue<W, ELEMENT, SIZE, Q, P>
where
    ELEMENT: FullCodec + Clone,
    Q: StorageValue<BoundedVec<ELEMENT, SIZE>, Query = BoundedVec<ELEMENT, SIZE>>,
    P: StorageValue<(u32, u32), Query = (u32, u32)>,
    SIZE: Get<u32>,
    W: Get<RuntimeDbWeight>,
{
    fn convert_logical_index_to_actual(
        logical_index: u32,
        queue_length: u32,
    ) -> Result<usize, QueueError> {
        if logical_index >= queue_length {
            return Err(QueueError::IndexOutOfRange);
        }
        let (head, _tail) = P::get();
        Ok(((head + logical_index) % queue_length) as usize)
    }

    pub fn init(initial_elements: BoundedVec<ELEMENT, SIZE>) -> Weight {
        Q::put(initial_elements);
        P::put((0, SIZE::get().saturating_sub(1)));
        Weight::from_parts(10000, 1) + W::get().reads_writes(0, 2)
    }

    pub fn overwrite_queue(
        map_fn: &dyn Fn(&mut ELEMENT) -> (bool, Weight),
        mut start_index: u32,
    ) -> Result<Weight, QueueError> {
        let mut queue = Q::get();
        let mut total_map_weight = Weight::zero();

        let initial_actual_index =
            Self::convert_logical_index_to_actual(start_index, queue.len() as u32)?;
        let (mut should_continue, mut current_map_weight) =
            map_fn(&mut queue[initial_actual_index]);
        total_map_weight += current_map_weight;

        while should_continue {
            start_index += 1;
            if let Ok(actual_index) =
                Self::convert_logical_index_to_actual(start_index, queue.len() as u32)
            {
                (should_continue, current_map_weight) = map_fn(&mut queue[actual_index]);
                total_map_weight += current_map_weight;
            } else {
                // It seems we have traversed through all elements
                break;
            }
        }
        Q::put(queue);

        Ok(Weight::from_parts(10000, 1) + W::get().reads_writes(2, 1) + total_map_weight)
    }

    pub fn tail() -> (ELEMENT, Weight) {
        let (_head, tail) = P::get();
        let queue = Q::get();
        (queue[tail as usize].clone(), W::get().reads(2))
    }

    pub fn head() -> (ELEMENT, Weight) {
        let (head, _tail) = P::get();
        let queue = Q::get();
        (queue[head as usize].clone(), W::get().reads(2))
    }

    pub fn element_at_index(index: u32) -> Result<(ELEMENT, Weight), QueueError> {
        let actual_index = Self::convert_logical_index_to_actual(index, SIZE::get())?;
        let queue = Q::get();
        Ok((queue[actual_index].clone(), W::get().reads(2)))
    }

    pub fn shift_queue() -> Weight {
        let (head, tail) = P::get();
        let size = SIZE::get();
        let mut queue = Q::get();

        // We need to increment head and tail
        let new_head = if head + 1 >= size { 0 } else { head + 1 };

        let new_tail = if tail + 1 >= size { 0 } else { tail + 1 };

        queue[new_tail as usize] = queue[tail as usize].clone();

        Q::put(queue);
        P::put((new_head, new_tail));

        Weight::from_parts(10000, 1) + W::get().reads_writes(2, 2)
    }
}
