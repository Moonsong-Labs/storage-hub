use codec::Encode;
use sp_inherents::{InherentIdentifier, IsFatalError};

#[cfg(feature = "std")]
use codec::Decode;
#[cfg(feature = "std")]
use sp_inherents::{Error, InherentData};

#[derive(Encode)]
#[cfg_attr(feature = "std", derive(Debug, Decode))]
pub enum InherentError {
    Other(alloc::string::String),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match *self {
            InherentError::Other(_) => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &*data).ok()
        } else {
            None
        }
    }
}

/// The InherentIdentifier to set the babe randomness results
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"baberand";

/// A bare minimum inherent data provider that provides no real data.
/// The inherent is simply used as a way to kick off some computation
/// until https://github.com/paritytech/substrate/pull/10128 lands.
pub struct InherentDataProvider;

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
    async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &())
    }

    async fn try_handle_error(
        &self,
        identifier: &InherentIdentifier,
        _error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        // Don't process modules from other inherents
        if *identifier != INHERENT_IDENTIFIER {
            return None;
        }

        // All errors with the randomness inherent are fatal
        Some(Err(Error::Application(Box::from(String::from(
            "Error processing dummy randomness inherent",
        )))))
    }
}
