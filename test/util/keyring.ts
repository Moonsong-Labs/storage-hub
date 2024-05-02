import type { PolkadotSigner, SS58String } from "polkadot-api";
import { fromHex } from "@polkadot-api/utils";
import { ed25519 } from "@noble/curves/ed25519";
import { sha256 } from "@noble/hashes/sha256";
import { Keyring } from "@polkadot/api";
import { waitReady } from "@polkadot/wasm-crypto";
import { blake2b } from "@noble/hashes/blake2b";
import { secp256k1 } from "@noble/curves/secp256k1";
import { getPolkadotSigner } from "polkadot-api/signer";
import { AccountId } from "polkadot-api";
import { toHex } from "polkadot-api/utils";

// These Keys have been generated via subkey util from polkadot-sdk
// e.g.:
// ❯ subkey inspect --scheme sr25519 --network substrate //Alice
// Secret Key URI `//Alice` is account:
//   Network ID:        substrate
//   Secret seed:       0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a
//   Public key (hex):  0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
//   Account ID:        0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
//   Public key (SS58): 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
//   SS58 Address:      5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY          ─╯

const people = [
  {
    name: "alice",
    sr25519Key: "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a",
    ed25519Key: "0xabf8e5bdbe30c65656c0a3cbd181ff8a56294a69dfedd27982aace4a76909115",
    ecdsaKey: "0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854",
  },
  {
    name: "bob",
    sr25519Key: "0x398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89",
    ed25519Key: "0x3b7b60af2abcd57ba401ab398f84f4ca54bd6b2140d2503fbcf3286535fe3ff1",
    ecdsaKey: "0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf",
  },
  {
    name: "charlie",
    sr25519Key: "0xbc1ede780f784bb6991a585e4f6e61522c14e1cae6ad0895fb57b9a205a8f938",
    ed25519Key: "0x072c02fa1409dc37e03a4ed01703d4a9e6bba9c228a49a00366e9630a97cba7c",
    ecdsaKey: "0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589",
  },
  {
    name: "dave",
    sr25519Key: "0x868020ae0687dda7d57565093a69090211449845a7e11453612800b663307246",
    ed25519Key: "0x771f47d3caf8a2ee40b0719e1c1ecbc01d73ada220cf08df12a00453ab703738",
    ecdsaKey: "0xfa6ba451077fecce7510092e307338e04150ffccc7224c13561a2b079935a5f7",
  },
  {
    name: "eve",
    sr25519Key: "0x786ad0e2df456fe43dd1f91ebca22e235bc162e0bb8d53c633e8c85b2af68b7a",
    ed25519Key: "0xbef5a3cd63dd36ab9792364536140e5a0cce6925969940c431934de056398556",
    ecdsaKey: "0x6b30a5e36f608b73e54665c094f97e221554157fcd03e8be7e25ad32f0e1e5b4",
  },
  {
    name: "ferdie",
    sr25519Key: "0x42438b7883391c05512a938e36c2df0131e088b3756d6aa7a755fbff19d2f842",
    ed25519Key: "0x1441e38eb309b66e9286867a5cd05902b05413eb9723a685d4d77753d73d0a1d",
    ecdsaKey: "0x1a02e99b89e0f7d3488d53ded5a3ef2cff6046543fc7f734206e3e842089e051",
  },
] as const;

type PeopleNames = (typeof people)[number]["name"];
type AccountType = Record<
  "sr25519" | "ecdsa" | "ed25519",
  { signer: PolkadotSigner; id: SS58String }
>;

const keyring = new Keyring({ type: "sr25519" });
await waitReady();

const signEcdsa = (value: Uint8Array, priv: Uint8Array) => {
  const signature = secp256k1.sign(blake2b(value), priv);
  const signedBytes = signature.toCompactRawBytes();

  const result = new Uint8Array(signedBytes.length + 1);
  result.set(signedBytes);
  result[signedBytes.length] = signature.recovery;

  return result;
};

const accountEntries = people.map((person) => {
  const keyringPair = keyring.addFromUri(
    `//${person.name.charAt(0).toUpperCase() + person.name.slice(1)}`
  );

  // Define the signer for ed25519
  const edPublicKey = ed25519.getPublicKey(fromHex(person.ed25519Key));
  const ed25519Signer = getPolkadotSigner(edPublicKey, "Ed25519", async (input) =>
    ed25519.sign(input, fromHex(person.ed25519Key))
  );

  // Define the signer for ecdsa
  const ecdsaPrivateKey = fromHex(person.ecdsaKey);
  const ecdsaSigner = getPolkadotSigner(
    blake2b(secp256k1.getPublicKey(ecdsaPrivateKey)),
    "Ecdsa",
    async (input) => signEcdsa(input, ecdsaPrivateKey)
  );

  return [
    person.name,
    {
      sr25519: {
        signer: getPolkadotSigner(keyringPair.publicKey, "Sr25519", async (input) =>
          keyringPair.sign(input)
        ),
        id: AccountId().dec(keyringPair.publicKey),
      },
      ed25519: {
        signer: ed25519Signer,
        id: AccountId().dec(edPublicKey),
      },
      ecdsa: {
        signer: ecdsaSigner,
        id: AccountId().dec(secp256k1.getPublicKey(ecdsaPrivateKey)),
      },
    },
  ];
});

const accounts: Record<PeopleNames, AccountType> = Object.fromEntries(accountEntries);

export { accounts };

export const getSr25519Account = async (privateKey?: string) => {
  const stringKey = privateKey || toHex(sha256(performance.now().toString()));

  const edPublicKey = ed25519.getPublicKey(fromHex(stringKey));
  const ed25519Signer = getPolkadotSigner(edPublicKey, "Ed25519", async (input) =>
    ed25519.sign(input, fromHex(stringKey))
  );
  const id = AccountId().dec(edPublicKey);

  return { privateKey: stringKey, publicKey: edPublicKey, signer: ed25519Signer, id };
};
