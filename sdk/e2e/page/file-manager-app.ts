import { FileManager } from '@storagehub-sdk/core';
import wasmInit from '@storagehub/wasm';

// Initialize WASM module first, then expose FileManager
let wasmInitialized = false;

async function initializeWasm() {
    if (!wasmInitialized) {
        try {
            console.log('🔧 Initializing WASM module...');
            await wasmInit();
            wasmInitialized = true;
            console.log('✅ WASM module initialized successfully');
        } catch (error) {
            console.error('❌ Failed to initialize WASM module:', error);
            throw error;
        }
    }
}

// Initialize WASM and expose FileManager
initializeWasm().then(() => {
    // Expose FileManager to window for E2E test access
    (window as any).FileManager = FileManager;
    (window as any).wasmReady = true;
    console.log('📦 FileManager exposed to window');
}).catch(error => {
    console.error('❌ WASM initialization failed:', error);
    (window as any).wasmError = error.message;
});

// Initialize UI
document.addEventListener('DOMContentLoaded', () => {
    console.log('🔢 FileManager E2E Test App initialized');

    // Set test mode indicator
    const isHeadless = window.navigator.webdriver || window.location.search.includes('headless');
    const testModeEl = document.getElementById('test-mode');
    if (testModeEl) {
        testModeEl.textContent = isHeadless ? 'HEADLESS' : 'HEADED';
    }

    console.log(`📊 Test mode: ${isHeadless ? 'HEADLESS' : 'HEADED'}`);
    console.log('🧪 FileManager ready for testing');
});

// Helper functions for test steps visualization
(window as any).updateTestStep = (stepNumber: number, status: 'active' | 'completed') => {
    const stepEl = document.getElementById(`step-${stepNumber}`);
    if (stepEl) {
        stepEl.className = `step-indicator ${status}`;
    }
};

(window as any).updateProgress = (percentage: number) => {
    const progressEl = document.getElementById('progress');
    if (progressEl) {
        progressEl.style.width = `${percentage}%`;
    }
};

// Initialize FileManager availability check
const checkFileManagerAvailability = () => {
    try {
        const isAvailable = typeof FileManager !== 'undefined';
        const hasCorrectMethods = typeof FileManager.prototype?.getFingerprint === 'function';

        console.log(`📦 FileManager availability: ${isAvailable}`);
        console.log(`🔧 FileManager methods: ${hasCorrectMethods}`);

        if (isAvailable && hasCorrectMethods) {
            (window as any).updateTestStep(1, 'completed');
            console.log('✅ FileManager class loaded successfully');
        } else {
            console.error('❌ FileManager not properly loaded');
        }

        return isAvailable && hasCorrectMethods;
    } catch (error) {
        console.error('❌ Error checking FileManager:', error);
        return false;
    }
};

// Auto-check FileManager when page loads
setTimeout(() => {
    checkFileManagerAvailability();
}, 100);