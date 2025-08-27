import { describe, it, expect } from 'vitest';
import { join } from 'node:path';
import { statSync, createReadStream } from 'node:fs';
import { Readable } from 'node:stream';
import { FileManager } from '../src/index.js';
const EXPECTED_FINGERPRINT_HEX = '0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970';

const resourcePath = (name: string) =>
    join(__dirname, '../../../docker/resource', name);

const TEST_FILE = 'adolphus.jpg'; // same file used in Rust merkle root tests

describe('FileManager fingerprint', () => {
    it('computes fingerprint matching expected root', async () => {
        const path = resourcePath(TEST_FILE);
        const fm = new FileManager({
            size: statSync(path).size,
            stream: () => Readable.toWeb(createReadStream(path)) as any,
        });

        // Compute fingerprint
        const fingerprint = await fm.getFingerprint();
        expect(Buffer.from(fingerprint.toU8a()).toString('hex')).toBe(
            EXPECTED_FINGERPRINT_HEX.slice(2)
        );
    });
}); 
