/// A struct that represents a file key challenge.
///
/// The challenge consists of a u64 stored in a fixed size array.
/// The u64 can be interpreted as the chunk within a file that is being challenged.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Default)]
pub struct FileKeyChallenge {
    pub challenge: [u8; 8],
}

impl AsRef<[u8]> for FileKeyChallenge {
    fn as_ref(&self) -> &[u8] {
        &self.challenge
    }
}

impl AsMut<[u8]> for FileKeyChallenge {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.challenge
    }
}

impl From<u64> for FileKeyChallenge {
    fn from(challenge: u64) -> Self {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&challenge.to_be_bytes());
        Self { challenge: bytes }
    }
}

impl From<&u64> for FileKeyChallenge {
    fn from(challenge: &u64) -> Self {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&challenge.to_be_bytes());
        Self { challenge: bytes }
    }
}

impl From<&[u8]> for FileKeyChallenge {
    fn from(bytes: &[u8]) -> Self {
        let mut challenge = [0u8; 8];
        challenge.copy_from_slice(bytes);
        Self { challenge }
    }
}

impl Into<u64> for FileKeyChallenge {
    fn into(self) -> u64 {
        u64::from_be_bytes(self.challenge)
    }
}
