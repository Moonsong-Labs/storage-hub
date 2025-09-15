/**
 * FileSystem contract accessors (Core)
 *
 * Provides a typed getter for the FileSystem contract using viem and the
 * strongly-typed ABI from src/abi/filesystem.
 */

import { getContract, type Address, type PublicClient, type WalletClient, type GetContractReturnType } from 'viem';
import { filesystemAbi } from '../abi/filesystem';

export { filesystemAbi };

export type EvmClient = PublicClient | WalletClient;

export type FileSystemContract<TClient extends EvmClient> = GetContractReturnType<typeof filesystemAbi, TClient>;

/**
 * Constant precompile address for FileSystem on StorageHub runtimes.
 * If a chain uses a different address, this constant should be updated accordingly.
 */
export const FILE_SYSTEM_PRECOMPILE_ADDRESS =
  '0x0000000000000000000000000000000000000064' as Address;

/**
 * Returns a viem contract instance bound to the FileSystem ABI at the precompile address.
 * - `client` can be a PublicClient (reads) or WalletClient (writes)
 */
export function getFileSystemContract<TClient extends EvmClient>(client: TClient): FileSystemContract<TClient> {
  return getContract({ address: FILE_SYSTEM_PRECOMPILE_ADDRESS, abi: filesystemAbi, client });
}
