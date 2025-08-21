export type WalletErrorCode = 'InvalidPrivateKey' | 'InvalidMnemonic';

export class WalletError extends Error {
    public readonly name = 'WalletError';
    public readonly code: WalletErrorCode;

    public constructor(code: WalletErrorCode, message?: string) {
        super(message ?? code);
        this.code = code;
    }
}


