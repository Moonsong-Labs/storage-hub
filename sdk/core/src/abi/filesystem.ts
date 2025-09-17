// Typed re-export of the FileSystem ABI for abitype/viem consumers.
import fileSystemAbiJson from './FileSystem.abi.json';

export const filesystemAbi = fileSystemAbiJson;

// Runtime guard: fail fast if the imported JSON isn't a valid ABI array
// (e.g., wrong path, malformed file). This yields a clearer error early.
if (!Array.isArray(filesystemAbi)) {
  throw new Error('Invalid FileSystem ABI: expected array');
}
