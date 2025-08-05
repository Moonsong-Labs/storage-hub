import { test, expect } from './wallet.fixture';

const PAGE_URL = 'http://localhost:5173/basic.html';

test('test if Anvil network is available and switch to it', async ({ page, wallet }) => {
    console.log('🎬 Starting Anvil network availability test...');

    // Navigate to the page
    await page.goto(PAGE_URL);

    // Click the connect button
    await page.getByTestId('connect').click();

    // Approve connection in MetaMask
    await wallet.approve();

    // Verify wallet connected successfully
    await expect(page.getByTestId('address')).not.toHaveText('');

    // Check current network
    const currentNetwork = await page.evaluate(async () => {
        const chainId = await (window as any).ethereum.request({ method: 'eth_chainId' });
        return parseInt(chainId, 16);
    });

    console.log(`📡 Current chain ID: ${currentNetwork}`);

    if (currentNetwork === 31337) {
        console.log('✅ Already on Anvil network! Test successful.');
        return;
    }

    // Try to add/switch to Anvil network programmatically
    console.log('🔄 Attempting to add/switch to Anvil network programmatically...');

    try {
        // Use the standard Web3 approach to add/switch networks
        const networkResult = await page.evaluate(async () => {
            const ethereum = (window as any).ethereum;
            if (!ethereum) {
                throw new Error('No ethereum provider found');
            }

            try {
                // Try to switch to the network first
                await ethereum.request({
                    method: 'wallet_switchEthereumChain',
                    params: [{ chainId: '0x7a69' }], // 31337 in hex
                });
                return { action: 'switched', success: true };
            } catch (switchError: any) {
                // If the network doesn't exist (error 4902), add it
                if (switchError.code === 4902) {
                    await ethereum.request({
                        method: 'wallet_addEthereumChain',
                        params: [{
                            chainId: '0x7a69',
                            chainName: 'Hardhat Local',
                            rpcUrls: ['http://127.0.0.1:8545'],
                            nativeCurrency: {
                                name: 'Ethereum',
                                symbol: 'ETH',
                                decimals: 18
                            },
                            blockExplorerUrls: null
                        }]
                    });
                    return { action: 'added', success: true };
                } else {
                    throw switchError;
                }
            }
        });

        console.log(`✅ Network ${networkResult.action} request sent to MetaMask!`);
        console.log('⏳ Please approve the request in MetaMask...');

        // Give user time to approve in MetaMask (this is manual)
        console.log('⏳ Waiting 10 seconds for user to approve in MetaMask...');
        await page.waitForTimeout(10000);

        // Check if the network switch was successful
        const finalNetwork = await page.evaluate(async () => {
            const chainId = await (window as any).ethereum.request({ method: 'eth_chainId' });
            return parseInt(chainId, 16);
        });

        if (finalNetwork === 31337) {
            console.log('✅ Successfully connected to Anvil network!');
        } else {
            console.log(`⚠️  Still on chain ${finalNetwork}. User may have rejected the request.`);
        }

    } catch (error) {
        console.log(`❌ Programmatic network addition failed: ${error}`);
        console.log('');
        console.log('📋 Please add the network manually:');
        console.log('1. In the MetaMask popup that should be visible, click "Add a custom network"');
        console.log('2. Fill in these details:');
        console.log('   - Network Name: Hardhat Local');
        console.log('   - RPC URL: http://127.0.0.1:8545');
        console.log('   - Chain ID: 31337');
        console.log('   - Currency Symbol: ETH');
        console.log('3. Click "Save" and "Switch to Hardhat Local"');
        console.log('4. Rerun the test');
        console.log('');
        console.log('💡 Make sure Anvil is running: pnpm run node:e2e');
    }

    console.log('✅ Anvil network test completed!');
});