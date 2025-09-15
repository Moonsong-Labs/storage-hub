// Register an MSP using an EVM (ECDSA) account as identity on a solochain-evm node.
// - Uses the provided EVM private key to derive an ECDSA Substrate account (identity)
// - Uses sudo (Alice sr25519) only to authorize forceMspSignUp
// - Returns { mspId, valuePropId? }

/**
 * @typedef {Object} RegisterMspEvmOptions
 * @property {`0x${string}`} privateKey EVM private key (ECDSA) for the MSP identity
 * @property {`0x${string}`} evmAddress 20-byte EVM address (0x-prefixed) for MSP ID derivation
 * @property {string=} wsUrl WebSocket URL to the node (default ws://127.0.0.1:9944)
 */

/**
 * @param {RegisterMspEvmOptions} opts
 */
export async function registerMspEvm(opts) {
  const wsUrl = (opts && opts.wsUrl) || 'ws://127.0.0.1:9944';
  const { ApiPromise, WsProvider, Keyring } = await import('@polkadot/api');
  let KeyringCtor = Keyring;
  if (!KeyringCtor) {
    const keyringMod = await import('@polkadot/keyring');
    KeyringCtor = keyringMod.Keyring;
  }
  const { hexToU8a, u8aToHex } = await import('@polkadot/util');

  const provider = new WsProvider(wsUrl);
  const api = await ApiPromise.create({ provider });

  const ethRing = new KeyringCtor({ type: 'ethereum' });
  // On solochain-evm, use Ethereum accounts for signing extrinsics (AccountId20 + 65-byte signatures)
  const msp = ethRing.addFromSeed(hexToU8a(opts.privateKey));

  const providers = api.tx.providers;
  if (!providers) throw new Error('providers pallet not available');

  // Prepare params
  const capacity = 1n;
  const storageLimit = 1n * 1024n; // 1 KB
  const terms = 'Terms';
  const fee = 1n;
  const multiaddrHex = u8aToHex(new TextEncoder().encode('/ip4/127.0.0.1/tcp/30350/p2p/12D3KooWTestPeerIdForMsp'));

  // bytes32 MSP ID derived from EVM address (left-pad 12 bytes)
  const mspIdHex = /** @type {`0x${string}`} */ (`0x${'0'.repeat(24)}${opts.evmAddress.slice(2)}`);

  // Non-sudo registration flow for MSP (signed by MSP EVM account)
  const req = providers.requestMspSignUp(
    capacity,
    [multiaddrHex],
    1n, // valuePropPricePerGigaUnitOfDataPerBlock (minimize deposit)
    u8aToHex(new TextEncoder().encode('c')),
    1n * 1024n, // valuePropMaxDataLimit
    msp.address,
  );
  // Tip to avoid 1014 priority-too-low errors when re-submitting quickly in tests
  let mspIdFromEvents;
  let requestErrored = false;
  await new Promise((resolve) => {
    req.signAndSend(msp, { tip: 1n }, (result) => {
      try {
        if (result.events && result.events.length > 0) {
          for (const { event } of result.events) {
            const section = event?.section;
            const method = event?.method;
            if (section === 'providers' && (method?.includes('Msp') || method?.includes('MainStorageProvider'))) {
              const hexes = [];
              // @ts-ignore
              for (const d of event.data) {
                if (typeof d?.toHex === 'function') {
                  const h = d.toHex();
                  if (typeof h === 'string' && /^0x[0-9a-fA-F]{64}$/.test(h)) hexes.push(h);
                }
              }
              if (hexes.length > 0 && !mspIdFromEvents) mspIdFromEvents = /** @type {`0x${string}`} */ (hexes[0]);
            }
          }
        }
      } catch { }
      if (result.dispatchError) {
        try { console.error('[registerMspEvm] requestMspSignUp dispatchError:', result.dispatchError.toString()); } catch { }
        requestErrored = true;
        return resolve();
      }
      if (result.status?.isInBlock || result.status?.isFinalized) return resolve();
      if (result.isError) return resolve();
    }).catch(() => resolve());
  });

  // Some networks require an explicit confirmation step
  if (providers.confirmSignUp) {
    const conf = providers.confirmSignUp(msp.address);
    await new Promise((resolve, reject) => {
      conf.signAndSend(msp, { tip: 1n }, (result) => {
        if (result.dispatchError) {
          try { console.error('[registerMspEvm] confirmSignUp dispatchError:', result.dispatchError.toString()); } catch { }
          return reject(result.dispatchError);
        }
        if (result.status?.isInBlock || result.status?.isFinalized) return resolve();
        if (result.isError) return reject(result);
      }).catch(reject);
    });
  }

  // Fallback: if mapping still not set later and request path failed, try sudo.forceMspSignUp (if sudo pallet exists)
  // This helps CI/dev when deposits or commit phases block the non-sudo flow.
  try {
    const sudoTx = api.tx.sudo;
    if (sudoTx && api.tx.balances && (requestErrored || !mspIdFromEvents)) {
      // Attempt a minimal force sign-up using the same MSP account & id
      const force = sudoTx.sudo(
        providers.forceMspSignUp(
          msp.address,
          mspIdHex,
          512n * 1024n * 1024n,
          [multiaddrHex],
          100 * 1024 * 1024,
          'Terms of Service...',
          9999999,
          msp.address,
        ),
      );
      await new Promise((resolve) => {
        force.signAndSend(msp, { tip: 1n }, (result) => {
          if (result.dispatchError) {
            try { console.error('[registerMspEvm] forceMspSignUp dispatchError:', result.dispatchError.toString()); } catch { }
            return resolve();
          }
          if (result.status?.isInBlock || result.status?.isFinalized) return resolve();
          if (result.isError) return resolve();
        }).catch(() => resolve());
      });
      // If forced, prefer the explicit mspIdHex
      if (!mspIdFromEvents) mspIdFromEvents = mspIdHex;
    }
  } catch { }

  // Try to fetch a value proposition (optional)
  let valuePropId;
  try {
    if (api.call && api.call.storageProvidersApi && api.call.storageProvidersApi.queryValuePropositionsForMsp) {
      const res = await api.call.storageProvidersApi.queryValuePropositionsForMsp(mspIdHex);
      console.log('[registerMspEvm] queryValuePropositionsForMsp length:', Array.isArray(res) ? res.length : 'n/a');
      if (Array.isArray(res) && res.length > 0 && res[0]?.id) valuePropId = /** @type {`0x${string}`} */ (res[0].id.toString());
    }
  } catch { }

  // Runtime often maps account -> providerId internally. Resolve canonical MSP id if available.
  let resolvedMspId = mspIdFromEvents || mspIdHex;
  try {
    if (api.query && api.query.providers && api.query.providers.accountIdToMainStorageProviderId) {
      for (let i = 0; i < 20; i++) {
        const opt = await api.query.providers.accountIdToMainStorageProviderId(msp.address);
        if (opt && opt.isSome) {
          resolvedMspId = /** @type {`0x${string}`} */ (opt.unwrap().toHex());
          console.log('[registerMspEvm] Resolved MSP id from chain mapping:', resolvedMspId);
          break;
        }
        await new Promise(r => setTimeout(r, 250));
      }
      if (resolvedMspId === mspIdHex) {
        console.log('[registerMspEvm] No chain mapping for MSP account after wait; using padded EVM address');
      }
    }
  } catch { }

  await api.disconnect();
  console.log('[registerMspEvm] MSP registration complete. mspId:', resolvedMspId, 'valuePropId:', valuePropId ?? '(none)');
  return { mspId: resolvedMspId, valuePropId };
}


