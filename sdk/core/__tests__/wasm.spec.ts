import { add } from '@storagehub/wasm';
import { describe, expect, it } from 'vitest';

describe('WASM bindings', () => {
    const cases: Array<[number, number, number]> = [
        [0, 0, 0],
        [1, 2, 3],
        [5, 7, 12],
        [100, 200, 300],
        [13, 29, 42],
    ];

    cases.forEach(([a, b, sum]) => {
        it(`adds ${a} + ${b} = ${sum}`, () => {
            expect(add(a, b)).toBe(sum);
        });
    });
}); 