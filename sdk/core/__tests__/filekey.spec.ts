import { describe, it, expect } from 'vitest';
import { FileMetadata } from '@storagehub/wasm';
import { TypeRegistry } from '@polkadot/types';
import type { AccountId, H256 } from '@polkadot/types/interfaces';

// Values captured from the Rust runtime test `bsp_confirm_storing_correctly_updates_already_existing_payment_stream`
const OWNER_HEX = '0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d';
const BUCKET_ID_HEX = '0x6d2ac7daaac165d4dd1fc1ec7bb9706d8c43c7545bbfcd8d24e3e802eacb44a1';
const LOCATION_STR = 'test';
const FILE_SIZE = 4n; // u64 in Rust
const FINGERPRINT = '0x0000000000000000000000000000000000000000000000000000000000000000';
const EXPECTED_FILEKEY = '0x15df3ac7a3169b0193ae13d15ac4c7e9337962d70fa87055fb46dc386492a786';

describe('FileKey generation (wasm wrapper)', () => {
    const registry = new TypeRegistry();
    const owner: AccountId = registry.createType('AccountId32', OWNER_HEX);
    const bucketId: H256 = registry.createType('H256', BUCKET_ID_HEX);
    const fingerprint: H256 = registry.createType('H256', FINGERPRINT);

    it('matches the expected runtime-generated FileKey', () => {
        const metadata = new FileMetadata(
            owner.toU8a(),
            bucketId.toU8a(),
            new TextEncoder().encode(LOCATION_STR),
            FILE_SIZE,
            fingerprint.toU8a()
        );

        const fileKey = '0x' + Buffer.from(metadata.getFileKey()).toString('hex');
        expect(fileKey).toBe(EXPECTED_FILEKEY);
    });
}); 