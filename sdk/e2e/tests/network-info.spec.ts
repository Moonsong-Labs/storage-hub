import { test, expect } from './wallet.fixture';

const PAGE_URL = 'http://localhost:5173/basic.html';

test('display current network information', async ({ page, wallet }) => {
    console.log('🎬 Starting network information test...');

    // Navigate to the page
    await page.goto(PAGE_URL);

    // Click the connect button
    await page.getByTestId('connect').click();

    // Approve connection in MetaMask
    await wallet.approve();

    // Verify wallet connected successfully
    await expect(page.getByTestId('address')).not.toHaveText('');

    // Get comprehensive network information
    const networkInfo = await page.evaluate(async () => {
        const ethereum = (window as any).ethereum;
        if (!ethereum) {
            return { error: 'No ethereum provider found' };
        }

        try {
            const [chainId, accounts] = await Promise.all([
                ethereum.request({ method: 'eth_chainId' }),
                ethereum.request({ method: 'eth_accounts' })
            ]);

            return {
                chainId,
                chainIdDecimal: parseInt(chainId, 16),
                accounts,
                provider: 'MetaMask'
            };
        } catch (error: any) {
            return { error: error.message };
        }
    });

    if (networkInfo.error) {
        console.log(`❌ Error getting network info: ${networkInfo.error}`);
        return;
    }

    console.log('\n📊 Current Network Information:');
    console.log(`   Chain ID (hex): ${networkInfo.chainId}`);
    console.log(`   Chain ID (decimal): ${networkInfo.chainIdDecimal}`);
    console.log(`   Connected Account: ${networkInfo.accounts[0]}`);
    console.log(`   Provider: ${networkInfo.provider}`);

    // Determine network name
    let networkName = 'Unknown';
    switch (networkInfo.chainIdDecimal) {
        case 1:
            networkName = 'Ethereum Mainnet';
            break;
        case 5:
            networkName = 'Goerli Testnet';
            break;
        case 11155111:
            networkName = 'Sepolia Testnet';
            break;
        case 31337:
            networkName = 'Anvil/Hardhat Local';
            break;
        case 1337:
            networkName = 'Ganache Local';
            break;
        default:
            networkName = `Custom Network (${networkInfo.chainIdDecimal})`;
    }

    console.log(`   Network: ${networkName}`);

    if (networkInfo.chainIdDecimal === 31337) {
        console.log('\n✅ Perfect! You are connected to the Anvil local network.');
        console.log('🚀 Your StorageHub SDK e2e tests are ready to go!');

        // Test basic RPC call to verify Anvil is working
        try {
            const blockNumber = await page.evaluate(async () => {
                return await (window as any).ethereum.request({
                    method: 'eth_blockNumber'
                });
            });
            console.log(`📦 Current block number: ${parseInt(blockNumber, 16)}`);
        } catch (e) {
            console.log('⚠️  Could not get block number');
        }
    } else {
        console.log('\n💡 To test with your local Anvil network:');
        console.log('1. Add a custom network in MetaMask with these details:');
        console.log('   - Network Name: Hardhat Local');
        console.log('   - RPC URL: http://127.0.0.1:8545');
        console.log('   - Chain ID: 31337');
        console.log('   - Currency Symbol: ETH');
        console.log('2. Switch to that network in MetaMask');
        console.log('3. Run this test again');
    }

    console.log('\n✅ Network information test completed!');
});