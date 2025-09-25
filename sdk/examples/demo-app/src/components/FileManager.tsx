'use client';

import { useState, useRef, useCallback, useEffect } from 'react';
import { Upload, Download, File, Folder, Hash, Info, X, CheckCircle, AlertCircle, Plus, Database } from 'lucide-react';
import { type WalletClient, type PublicClient, formatEther } from 'viem';
import { FileManager as StorageHubFileManager, initWasm, StorageHubClient, ReplicationLevel } from '@storagehub-sdk/core';
import { MspClient, type UploadReceipt, type DownloadResult, type Bucket, type FileListResponse } from '@storagehub-sdk/msp-client';
import { TypeRegistry } from '@polkadot/types';
import type { AccountId20, H256 } from '@polkadot/types/interfaces';

interface FileManagerProps {
  walletClient: WalletClient | null;
  publicClient: PublicClient | null;
  walletAddress: string | null;
  mspClient: MspClient | null;
  storageHubClient: StorageHubClient | null;
}

interface FileUploadState {
  file: File | null;
  fingerprint: string | null;
  isComputing: boolean;
  isUploading: boolean;
  uploadProgress: number;
  error: string | null;
  success: boolean;
  receipt: UploadReceipt | null;
}

interface BucketCreationState {
  bucketName: string;
  isCreating: boolean;
  error: string | null;
  success: boolean;
  createdBucketId: string | null;
}

export function FileManager({ walletClient, publicClient, walletAddress, mspClient, storageHubClient }: FileManagerProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  
  const [uploadState, setUploadState] = useState<FileUploadState>({
    file: null,
    fingerprint: null,
    isComputing: false,
    isUploading: false,
    uploadProgress: 0,
    error: null,
    success: false,
    receipt: null
  });

  const [bucketState, setBucketState] = useState<BucketCreationState>({
    bucketName: '',
    isCreating: false,
    error: null,
    success: false,
    createdBucketId: null
  });

  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [selectedBucketId, setSelectedBucketId] = useState<string>('');
  const [walletBalance, setWalletBalance] = useState<string | null>(null);
  const [isLoadingBuckets, setIsLoadingBuckets] = useState<boolean>(false);

  // Get wallet balance
  useEffect(() => {
    const getBalance = async () => {
      if (publicClient && walletAddress) {
        try {
          const balance = await publicClient.getBalance({ address: walletAddress as `0x${string}` });
          setWalletBalance(formatEther(balance));
        } catch (error) {
          console.error('Failed to get balance:', error);
        }
      }
    };
    getBalance();
  }, [publicClient, walletAddress]);

  // File selection handler
  const handleFileSelect = useCallback(async (file: File) => {
    setUploadState(prev => ({
      ...prev,
      file,
      fingerprint: null,
      error: null,
      success: false,
      receipt: null
    }));

    // Compute fingerprint
    setUploadState(prev => ({ ...prev, isComputing: true }));

    try {
      await initWasm();

      const fileManager = new StorageHubFileManager({
        size: file.size,
        stream: () => {
          return new ReadableStream<Uint8Array>({
            start(controller) {
              const reader = new FileReader();
              reader.onload = () => {
                const arrayBuffer = reader.result as ArrayBuffer;
                const uint8Array = new Uint8Array(arrayBuffer);
                controller.enqueue(uint8Array);
                controller.close();
              };
              reader.onerror = () => controller.error(reader.error);
              reader.readAsArrayBuffer(file);
            }
          });
        }
      });

      const fingerprint = await fileManager.getFingerprint();

      setUploadState(prev => ({
        ...prev,
        fingerprint: fingerprint.toHex(),
        isComputing: false
      }));
    } catch (error) {
      console.error('Fingerprint computation failed:', error);
      setUploadState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to compute fingerprint',
        isComputing: false
      }));
    }
  }, []);

  // Bucket creation function
  const createBucket = async () => {
    if (!bucketState.bucketName.trim() || !storageHubClient || !walletAddress || !publicClient) return;

    setBucketState(prev => ({ ...prev, isCreating: true, error: null }));

    try {
      const TEST_MSP_ID = '0x0000000000000000000000000000000000000000000000000000000000000300' as `0x${string}`;
      const TEST_VALUE_PROP_ID = '0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b' as `0x${string}`;

      console.log('🔍 Creating bucket with test constants:', {
        mspId: TEST_MSP_ID,
        valuePropId: TEST_VALUE_PROP_ID,
        bucketName: bucketState.bucketName,
        walletAddress,
        balance: walletBalance
      });

      const bucketId = await storageHubClient.deriveBucketId(walletAddress as `0x${string}`, bucketState.bucketName);
      console.log('Derived bucket ID:', bucketId);

          const txHash = await storageHubClient.createBucket(
            TEST_MSP_ID,
            bucketState.bucketName,
            false, // isPrivate
            TEST_VALUE_PROP_ID,
            {
              // Explicit gas settings to avoid estimation issues
              gas: BigInt(500000), // Explicit gas limit
              gasPrice: BigInt('1000000000') // 1 gwei
            }
          );

      console.log('Bucket creation transaction submitted:', txHash);

      const receipt = await publicClient!.waitForTransactionReceipt({ hash: txHash });

      if (receipt.status === 'success') {
        setBucketState(prev => ({
          ...prev,
          isCreating: false,
          success: true,
          createdBucketId: bucketId as string,
          error: null
        }));

        console.log('🎉 Bucket creation completed successfully!');
        
        // Refresh bucket list from MSP backend to get the latest state
        console.log('🔄 Refreshing bucket list after creation...');
        await loadBuckets();
      } else {
        throw new Error('Bucket creation transaction failed');
      }
        } catch (error: any) {
          console.error('Bucket creation failed:', error);
          setBucketState(prev => ({
            ...prev,
            error: error instanceof Error ? error.message : 'Bucket creation failed',
        isCreating: false
      }));
    }
  };

  // Load buckets from MSP backend
  const loadBuckets = async () => {
    if (!mspClient) {
      console.warn('⚠️ MSP client not available, cannot load buckets');
      return;
    }
    
    setIsLoadingBuckets(true);
    
    try {
      console.log('🔄 Refreshing buckets from MSP backend...');
      
      // DEBUGGING: Check if token is set on MSP client
      const clientToken = (mspClient as any).token;
      console.log('🔍 DEBUG: MSP Client token set?', !!clientToken);
      if (clientToken) {
        console.log('🔍 DEBUG: Token preview:', clientToken.substring(0, 50) + '...');
        
        // Decode the JWT payload to verify contents
        try {
          const parts = clientToken.split('.');
          if (parts.length === 3) {
            const payload = JSON.parse(atob(parts[1] + '=='));
            console.log('🔍 DEBUG: JWT payload:', payload);
          }
        } catch (decodeError) {
          console.log('🔍 DEBUG: Could not decode JWT:', decodeError);
        }
      } else {
        console.error('❌ DEBUG: No token found on MSP client!');
      }
      
      // DEBUGGING: Test MSP client health first (no auth required)
      console.log('🔍 DEBUG: Testing MSP client health...');
      try {
        const health = await mspClient.getHealth();
        console.log('🔍 DEBUG: MSP health check:', health);
      } catch (healthError) {
        console.error('🔍 DEBUG: MSP health check failed:', healthError);
      }
      
      console.log('🔄 Making listBuckets request...');
      
      // DEBUGGING: Test the HTTP client directly (EXACTLY like curl command)
      console.log('🔍 DEBUG: Testing HTTP client directly (like curl)...');
      console.log('🔍 DEBUG: Using exact same JWT as working curl:', clientToken);
      
      // DEBUGGING: Check what address is in the JWT vs wallet
      try {
        const jwtParts = clientToken.split('.');
        const payload = JSON.parse(atob(jwtParts[1] + '=='));
        console.log('🔍 DEBUG: JWT token address:', payload.address);
        console.log('🔍 DEBUG: Current wallet address:', walletAddress);
        console.log('🔍 DEBUG: Addresses match?', payload.address?.toLowerCase() === walletAddress?.toLowerCase());
      } catch (e) {
        console.log('🔍 DEBUG: Could not decode JWT for address comparison');
      }
      console.log('🔍 DEBUG: Making request to: http://127.0.0.1:8080/buckets');
      console.log('🔍 DEBUG: Headers: Authorization: Bearer [token], Content-Type: application/json');
      
      try {
        const directResponse = await fetch('http://127.0.0.1:8080/buckets', {
          method: 'GET',
          headers: {
            'Authorization': `Bearer ${clientToken}`,
            'Content-Type': 'application/json'
          }
        });
        console.log('🔍 DEBUG: Direct fetch response status:', directResponse.status);
        console.log('🔍 DEBUG: Direct fetch response headers:', Object.fromEntries(directResponse.headers.entries()));
        
        if (directResponse.ok) {
          const directData = await directResponse.json();
          console.log('🔍 DEBUG: Direct fetch SUCCESS - data:', directData);
          console.log('🔍 DEBUG: Direct fetch found', directData.length, 'buckets');
        } else {
          const errorText = await directResponse.text();
          console.error('🔍 DEBUG: Direct fetch HTTP error:', directResponse.status, errorText);
        }
      } catch (directError) {
        console.error('🔍 DEBUG: Direct fetch NETWORK error:', directError);
      }
      
      // DEBUGGING: Intercept fetch to see what SDK is actually sending
      const originalFetch = window.fetch;
      const interceptedRequests: Array<{url: string, options: RequestInit}> = [];
      
      window.fetch = async (url: string | URL | Request, options?: RequestInit) => {
        const urlString = url.toString();
        if (urlString.includes('/buckets')) {
          console.log('🔍 DEBUG: INTERCEPTED SDK REQUEST:', urlString);
          console.log('🔍 DEBUG: INTERCEPTED SDK OPTIONS:', options);
          console.log('🔍 DEBUG: INTERCEPTED SDK HEADERS:', options?.headers);
          interceptedRequests.push({url: urlString, options: options || {}});
        }
        return originalFetch(url as any, options);
      };
      
      // DEBUGGING: Log the actual HTTP request being made
      console.log('🔍 DEBUG: About to call mspClient.listBuckets()...');
      console.log('🔍 DEBUG: MSP client config:', (mspClient as any).config);
      console.log('🔍 DEBUG: MSP client HTTP instance:', (mspClient as any).http);
      
      // DEBUGGING: Check the HttpClient's fetch implementation
      const httpClient = (mspClient as any).http;
      console.log('🔍 DEBUG: HttpClient fetchImpl:', typeof httpClient.fetchImpl);
      console.log('🔍 DEBUG: HttpClient baseUrl:', httpClient.baseUrl);
      console.log('🔍 DEBUG: HttpClient defaultHeaders:', httpClient.defaultHeaders);
      console.log('🔍 DEBUG: Global fetch available:', typeof globalThis.fetch);
      console.log('🔍 DEBUG: Window fetch available:', typeof window.fetch);
      
      // DEBUGGING: Test HttpClient directly
      console.log('🔍 DEBUG: Testing HttpClient directly...');
      try {
        const directHttpResult = await httpClient.get('/buckets', {
          headers: {
            'Authorization': `Bearer ${clientToken}`,
            'Content-Type': 'application/json'
          }
        });
        console.log('✅ Direct HttpClient SUCCESS:', directHttpResult);
      } catch (directHttpError: any) {
        console.error('❌ Direct HttpClient FAILED:', directHttpError);
        console.error('❌ Direct HttpClient error type:', typeof directHttpError);
        console.error('❌ Direct HttpClient error name:', directHttpError?.name);
        console.error('❌ Direct HttpClient error message:', directHttpError?.message);
      }
      
      console.log('🔍 DEBUG: ==========================================');
      console.log('🔍 DEBUG: NOW TESTING SDK vs DIRECT COMPARISON');
      console.log('🔍 DEBUG: ==========================================');
      
      let bucketList: any[] = [];
      try {
        console.log('🔍 DEBUG: Calling mspClient.listBuckets()...');
        bucketList = await mspClient.listBuckets();
        console.log('✅ Buckets loaded from MSP SDK:', bucketList);
      } catch (sdkError: any) {
        console.error('❌ SDK ERROR: mspClient.listBuckets() failed:', sdkError);
        console.error('❌ SDK ERROR type:', typeof sdkError);
        console.error('❌ SDK ERROR name:', sdkError?.name);
        console.error('❌ SDK ERROR message:', sdkError?.message);
        console.error('❌ SDK ERROR stack:', sdkError?.stack);
        bucketList = []; // Fallback to empty array
      }
      console.log('🔍 DEBUG: SDK result type:', typeof bucketList);
      console.log('🔍 DEBUG: SDK result Array.isArray:', Array.isArray(bucketList));
      console.log('🔍 DEBUG: SDK result length:', bucketList?.length);
      
      console.log('🔍 DEBUG: ==========================================');
      console.log('🔍 DEBUG: COMPARISON SUMMARY:');
      console.log('🔍 DEBUG: - Direct fetch (like curl): Should show buckets above');
      console.log('🔍 DEBUG: - SDK listBuckets():', bucketList?.length || 0, 'buckets');
      console.log('🔍 DEBUG: ==========================================');
      
      // Restore original fetch
      window.fetch = originalFetch;
      
      console.log('🔍 DEBUG: INTERCEPTED REQUESTS:', interceptedRequests);
      
      if (Array.isArray(bucketList) && bucketList.length === 0) {
        console.error('🚨 FOUND THE ISSUE: SDK returns empty array but direct fetch works!');
        console.error('🚨 This means the SDK HttpClient is not sending the request correctly!');
        console.error('🚨 Check intercepted requests above to see the difference!');
      }
      
      // Replace all buckets with the fresh list from MSP backend
      // This ensures we have the most up-to-date bucket information
      const freshBuckets = bucketList || [];
      setBuckets(freshBuckets);
      
      console.log(`📋 Updated bucket list: ${freshBuckets.length} buckets available`);
      freshBuckets.forEach(bucket => {
        console.log(`  - ${bucket.name} (ID: ${bucket.bucketId.slice(0, 8)}..., Files: ${bucket.fileCount}, Size: ${bucket.sizeBytes} bytes)`);
        console.log(`    🔍 DEBUG: Full bucket ID: "${bucket.bucketId}" (length: ${bucket.bucketId.length})`);
      });
      
    } catch (error: any) {
      console.error('❌ Failed to refresh buckets from MSP:', error);
      
      // Additional debugging for authentication errors
      if (error?.status === 401) {
        console.error('🔍 DEBUG: 401 Unauthorized - JWT token issue');
      } else if (error?.status === 403) {
        console.error('🔍 DEBUG: 403 Forbidden - Permission issue');
      } else if (error?.status) {
        console.error('🔍 DEBUG: HTTP Status:', error.status);
      }
      
      // Log response body if available
      if (error?.response) {
        console.error('🔍 DEBUG: Response body:', error.response);
      }
      
      // Show user-friendly error message
      console.error('This could be due to authentication issues or MSP backend connectivity problems');
    } finally {
      setIsLoadingBuckets(false);
    }
  };

  // Note: loadBuckets is only called manually via refresh button or after bucket creation
  // No automatic loading to avoid excessive API calls

  // File upload function
  const uploadFile = async () => {
    if (!uploadState.file || !uploadState.fingerprint || !mspClient || !storageHubClient || !walletAddress || !selectedBucketId) return;

    setUploadState(prev => ({ ...prev, isUploading: true, error: null }));

    try {
      await initWasm();

      const fileLocation = `/${uploadState.file.name}`;

      // Ensure file size is valid
      if (!uploadState.file.size || uploadState.file.size <= 0) {
        throw new Error(`Invalid file size: ${uploadState.file.size}`);
      }

      // Create FileManager to get fingerprint and compute file key
      const fileManager = new StorageHubFileManager({
        size: uploadState.file.size,
        stream: () => {
          return new ReadableStream<Uint8Array>({
            start(controller) {
              const reader = new FileReader();
              reader.onload = () => {
                const arrayBuffer = reader.result as ArrayBuffer;
                const uint8Array = new Uint8Array(arrayBuffer);
                controller.enqueue(uint8Array);
                controller.close();
              };
              reader.onerror = () => controller.error(reader.error);
              reader.readAsArrayBuffer(uploadState.file!);
            }
          });
        }
      });

      // Get file info from FileManager (like sdk-precompiles)
      const fingerprint = await fileManager.getFingerprint();
      const fileSizeNumber = fileManager.getFileSize();
      console.log('🔍 File size from FileManager:', fileSizeNumber);
      
      if (fileSizeNumber === undefined || fileSizeNumber === null) {
        throw new Error(`FileManager.getFileSize() returned ${fileSizeNumber}`);
      }
      
      const fileSize = BigInt(fileSizeNumber);

      // Create TypeRegistry and types for file key computation (like sdk-precompiles)
      const registry = new TypeRegistry();
      const owner = registry.createType("AccountId20", walletAddress) as AccountId20;
      
      // DEBUGGING: Check bucket ID format
      console.log('🔍 DEBUG: selectedBucketId:', selectedBucketId);
      console.log('🔍 DEBUG: selectedBucketId length:', selectedBucketId.length);
      console.log('🔍 DEBUG: selectedBucketId starts with 0x:', selectedBucketId.startsWith('0x'));
      
      // Ensure bucket ID is properly formatted as 32-byte hex string
      let bucketIdForH256 = selectedBucketId;
      if (!bucketIdForH256.startsWith('0x')) {
        bucketIdForH256 = '0x' + bucketIdForH256;
      }
      // H256 expects exactly 64 hex chars (32 bytes) after 0x
      if (bucketIdForH256.length !== 66) { // 0x + 64 hex chars = 66 total
        console.error('❌ Invalid bucket ID length for H256:', bucketIdForH256.length, 'expected 66');
        throw new Error(`Invalid bucket ID format: ${bucketIdForH256} (length: ${bucketIdForH256.length})`);
      }
      
      const bucketIdH256 = registry.createType("H256", bucketIdForH256) as H256;
      const fileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      console.log('📋 File metadata computed:', {
        owner: walletAddress,
        bucketId: selectedBucketId,
        location: fileLocation,
        fingerprint: fingerprint.toHex(),
        fileKey: fileKey.toHex(),
        fileSize: fileSize.toString()
      });

      // Additional debugging for MSP upload
      console.log('🔍 Upload parameters for MSP:');
      console.log('- selectedBucketId:', selectedBucketId);
      console.log('- fileKey.toHex():', fileKey.toHex());
      console.log('- walletAddress:', walletAddress);
      console.log('- fileLocation:', fileLocation);

      setUploadState(prev => ({ ...prev, uploadProgress: 25 }));

      // Issue storage request
      const TEST_MSP_ID = '0x0000000000000000000000000000000000000000000000000000000000000300';
      const MSP_PEER_ID = '12D3KooWSUvz8QM5X4tfAaSLErAZjR2puojo16pULBHyqTMGKtNV'; // MSP1 peer ID from consts (hardcoded)

      // EXTENSIVE DEBUGGING - Check every single parameter
      console.log('🔍 DEBUGGING ALL PARAMETERS:');
      console.log('selectedBucketId:', selectedBucketId, typeof selectedBucketId);
      console.log('fileLocation:', fileLocation, typeof fileLocation);
      console.log('fingerprint object:', fingerprint);
      console.log('fingerprint.toHex():', fingerprint.toHex(), typeof fingerprint.toHex());
      console.log('fileSize BigInt:', fileSize, typeof fileSize);
      console.log('TEST_MSP_ID:', TEST_MSP_ID, typeof TEST_MSP_ID);
      console.log('MSP_PEER_ID:', MSP_PEER_ID, typeof MSP_PEER_ID);
      console.log('MSP_PEER_ID length:', MSP_PEER_ID.length);
      console.log('MSP_PEER_ID starts with 12D3Koo:', MSP_PEER_ID.startsWith('12D3Koo'));
      console.log('ReplicationLevel.Basic:', ReplicationLevel.Basic, typeof ReplicationLevel.Basic);
      
      // Check if any are undefined
      const params: Array<{ name: string; value: any }> = [
        { name: 'selectedBucketId', value: selectedBucketId },
        { name: 'fileLocation', value: fileLocation },
        { name: 'fingerprint.toHex()', value: fingerprint.toHex() },
        { name: 'fileSize', value: fileSize },
        { name: 'TEST_MSP_ID', value: TEST_MSP_ID },
        { name: 'MSP_PEER_ID', value: MSP_PEER_ID },
        { name: 'ReplicationLevel.Basic', value: ReplicationLevel.Basic }
      ];
      
      params.forEach(param => {
        if (param.value === undefined) {
          console.error(`❌ FOUND UNDEFINED PARAMETER: ${param.name}`);
        }
      });

      console.log('🔧 Letting StorageHub client estimate gas automatically (like sdk-precompiles)');

      let storageRequestTxHash;
      try {
        console.log('🚀 STEP 1: Issuing storage request...');
        
        // DEBUGGING: Check bucket ID format for storage request
        console.log('🔍 DEBUG: selectedBucketId for storage request:', selectedBucketId);
        console.log('🔍 DEBUG: selectedBucketId length:', selectedBucketId.length);
        
        // Ensure bucket ID has 0x prefix for storage request
        const bucketIdForStorageRequest = selectedBucketId.startsWith('0x') ? selectedBucketId : `0x${selectedBucketId}`;
        console.log('🔍 DEBUG: bucketIdForStorageRequest:', bucketIdForStorageRequest);
        console.log('🔍 DEBUG: bucketIdForStorageRequest length:', bucketIdForStorageRequest.length);
        
        storageRequestTxHash = await storageHubClient.issueStorageRequest(
          bucketIdForStorageRequest as `0x${string}`,
          fileLocation,
          fingerprint.toHex() as `0x${string}`, // Use hex string like sdk-precompiles
          fileSize,
          TEST_MSP_ID as `0x${string}`,
          [MSP_PEER_ID],
          ReplicationLevel.Basic,
          0 // replicas (used only when ReplicationLevel = Custom, like sdk-precompiles)
          // No gas options - let it estimate naturally like sdk-precompiles
        );
        
        console.log('✅ STEP 1 SUCCESS: Storage request submitted:', storageRequestTxHash);
      } catch (error: any) {
        console.error('❌ STEP 1 FAILED: issueStorageRequest error:');
        console.error('Error message:', error?.message);
        console.error('Error stack:', error?.stack);
        console.error('Full error object:', error);
        throw error; // Re-throw to maintain the original behavior
      }

      console.log('🔄 STEP 2: Waiting for transaction receipt...');
      const storageRequestReceipt = await publicClient!.waitForTransactionReceipt({ 
        hash: storageRequestTxHash 
      });

      if (storageRequestReceipt.status !== 'success') {
        console.error('❌ STEP 2 FAILED: Storage request transaction failed');
        throw new Error('Storage request transaction failed');
      }

      console.log('✅ STEP 2 SUCCESS: Storage request transaction confirmed');
      setUploadState(prev => ({ ...prev, uploadProgress: 30 }));

      // CRITICAL: Recompute file key AFTER storage request (like sdk-precompiles line 215)
      console.log('🔄 STEP 3: Recomputing file key after storage request (sdk-precompiles pattern)...');
      const finalFileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);
      
      console.log('🔍 STEP 3 DEBUG: File key comparison:');
      console.log('- Original fileKey.toHex():', fileKey.toHex());
      console.log('- Final fileKey.toHex():', finalFileKey.toHex());
      console.log('- Keys match:', fileKey.toHex() === finalFileKey.toHex());

      // STEP 3.5: Wait a moment for MSP to process the storage request (like sdk-precompiles)
      console.log('⏳ STEP 3.5: Waiting for MSP to process storage request...');
      await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2 seconds
      setUploadState(prev => ({ ...prev, uploadProgress: 40 }));

      let uploadReceipt;
      try {
        console.log('🚀 STEP 4: Starting MSP file upload...');
        
        // Upload file to MSP (use exact same pattern as sdk-precompiles line 245-251)
        const fileBlob = await fileManager.getFileBlob(); // Get Blob like sdk-precompiles
        console.log('📁 File blob size:', fileBlob.size);
        
        const fileKeyHex = finalFileKey.toHex();
        console.log('📤 About to call uploadFile with:');
        console.log('- bucketId:', selectedBucketId);
        console.log('- fileKey:', fileKeyHex);
        console.log('- fileKey length:', fileKeyHex.length);
        console.log('- fileKey starts with 0x:', fileKeyHex.startsWith('0x'));
        console.log('- owner:', walletAddress);
        console.log('- location:', fileLocation);
        
        await new Promise(resolve => setTimeout(resolve, 3000)); // Add a 3 second delay before uploading
            // DEBUGGING: Check bucket ID format for MSP upload
            console.log('🔍 DEBUG: selectedBucketId for MSP upload:', selectedBucketId);
            console.log('🔍 DEBUG: selectedBucketId type:', typeof selectedBucketId);
            console.log('🔍 DEBUG: selectedBucketId length:', selectedBucketId.length);
            
            uploadReceipt = await mspClient.uploadFile(
              selectedBucketId, // MSP expects bucket ID without 0x prefix
              fileKeyHex, // Use the final computed file key
              fileBlob, // Use Blob instead of File object
              walletAddress, // owner parameter like sdk-precompiles
              fileLocation // location parameter like sdk-precompiles
            );
        
        console.log('✅ STEP 4 SUCCESS: MSP upload completed:', uploadReceipt);
        
      } catch (error: any) {
        console.error('❌ STEP 4 FAILED: MSP upload error:');
        console.error('Error message:', error?.message);
        console.error('Error stack:', error?.stack);
        console.error('Full error object:', error);
        
        // Additional debugging for HTTP errors
        if (error?.response) {
          console.error('HTTP Response Status:', error.response.status);
          console.error('HTTP Response Headers:', error.response.headers);
          console.error('HTTP Response Data:', error.response.data);
        }
        
        throw error; // Re-throw to maintain the original behavior
      }

      console.log('🎉 UPLOAD COMPLETE: All steps successful!');
      setUploadState(prev => ({
        ...prev,
        isUploading: false,
        success: true,
        receipt: uploadReceipt,
        error: null,
        uploadProgress: 100
      }));

        } catch (error: any) {
          console.error('Upload failed:', error);
          setUploadState(prev => ({
            ...prev,
            error: error instanceof Error ? error.message : 'Upload failed',
        isUploading: false
      }));
    }
  };

  const clearUpload = () => {
    setUploadState({
      file: null,
      fingerprint: null,
      isComputing: false,
      isUploading: false,
      uploadProgress: 0,
      error: null,
      success: false,
      receipt: null
    });
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <div className="space-y-6">
      {/* Bucket Creation Section */}
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Database className="h-5 w-5 text-blue-400" />
          <h3 className="text-lg font-medium">Create Bucket</h3>
        </div>
        
        <div className="flex gap-3">
          <input
            type="text"
            placeholder="Enter bucket name"
            value={bucketState.bucketName}
            onChange={(e) => setBucketState(prev => ({ ...prev, bucketName: e.target.value }))}
            className="flex-1 rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 placeholder-gray-400 focus:border-blue-500 focus:outline-none"
          />
          <button
            onClick={createBucket}
            disabled={!bucketState.bucketName.trim() || bucketState.isCreating}
            className="flex items-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
          >
            <Plus className="h-4 w-4" />
            {bucketState.isCreating ? 'Creating...' : 'Create Bucket'}
          </button>
        </div>

        {bucketState.error && (
          <div className="flex items-center gap-2 rounded-md bg-red-900/20 border border-red-900/50 p-3 text-red-400">
            <AlertCircle className="h-4 w-4" />
            <span className="text-sm">{bucketState.error}</span>
          </div>
        )}

        {bucketState.success && bucketState.createdBucketId && (
          <div className="flex items-center gap-2 rounded-md bg-green-900/20 border border-green-900/50 p-3 text-green-400">
            <CheckCircle className="h-4 w-4" />
            <span className="text-sm">Bucket created successfully! ID: {bucketState.createdBucketId.slice(0, 8)}...</span>
          </div>
        )}
      </div>

      {/* File Upload Section */}
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Upload className="h-5 w-5 text-blue-400" />
          <h3 className="text-lg font-medium">Upload File</h3>
        </div>

        {/* Bucket Selection */}
        <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Select Bucket ({buckets.length} available)
                {isLoadingBuckets && (
                  <span className="text-xs text-blue-400 ml-2 animate-pulse">
                    Refreshing...
                  </span>
                )}
                {!isLoadingBuckets && buckets.length === 0 && (
                  <span className="text-xs text-yellow-400 ml-2">
                    Click refresh to load buckets
                  </span>
                )}
                {!isLoadingBuckets && buckets.length > 0 && (
                  <span className="text-xs text-gray-500 ml-2">
                    [{buckets.map(b => `${b.name} (${b.fileCount} files)`).join(', ')}]
                  </span>
                )}
              </label>
          <div className="flex gap-3">
            <select
              value={selectedBucketId}
              onChange={(e) => setSelectedBucketId(e.target.value)}
              className="flex-1 rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 focus:outline-none"
            >
              <option value="">Select a bucket...</option>
                  {buckets.length === 0 && (
                    <option value="" disabled>No buckets available</option>
                  )}
                  {buckets.map((bucket) => {
                    console.log('🔍 Rendering bucket option:', bucket);
                    console.log('🔍 Bucket ID for dropdown:', bucket.bucketId, 'length:', bucket.bucketId.length);
                    return (
                      <option key={bucket.bucketId} value={bucket.bucketId}>
                        {bucket.name} ({bucket.bucketId.slice(0, 8)}...)
                      </option>
                    );
                  })}
            </select>
                <button
                  onClick={loadBuckets}
                  disabled={isLoadingBuckets}
                  className="px-4 py-2 text-sm bg-gray-700 text-gray-300 rounded-md hover:bg-gray-600 disabled:bg-gray-800 disabled:cursor-not-allowed transition-colors"
                >
                  {isLoadingBuckets ? 'Loading...' : 'Refresh'}
                </button>
          </div>
        </div>

        {/* File Selection */}
        <div className="space-y-3">
          <input
            ref={fileInputRef}
            type="file"
            onChange={(e) => {
              const files = e.target.files;
              if (files && files.length > 0) {
                handleFileSelect(files[0]);
              }
            }}
            className="block w-full text-sm text-gray-400 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-medium file:bg-blue-600 file:text-white hover:file:bg-blue-700"
          />

          {uploadState.file && (
            <div className="rounded-md bg-gray-800 p-4 space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <File className="h-4 w-4 text-blue-400" />
                  <span className="text-sm font-medium">{uploadState.file.name}</span>
                  <span className="text-xs text-gray-400">({(uploadState.file.size / 1024).toFixed(1)} KB)</span>
                </div>
                <button
                  onClick={clearUpload}
                  className="text-gray-400 hover:text-red-400"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>

              {uploadState.isComputing && (
                <div className="flex items-center gap-2 text-blue-400">
                  <Hash className="h-4 w-4 animate-spin" />
                  <span className="text-sm">Computing fingerprint...</span>
                </div>
              )}

              {uploadState.fingerprint && (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-green-400">
                    <CheckCircle className="h-4 w-4" />
                    <span className="text-sm">Fingerprint computed</span>
                  </div>
                  <div className="text-xs text-gray-400 font-mono break-all">
                    {uploadState.fingerprint}
                  </div>
                </div>
              )}

              {uploadState.fingerprint && selectedBucketId && (
                <button
                  onClick={uploadFile}
                  disabled={uploadState.isUploading}
                  className="w-full flex items-center justify-center gap-2 rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
                >
                  <Upload className="h-4 w-4" />
                  {uploadState.isUploading ? `Uploading... ${uploadState.uploadProgress}%` : 'Upload File'}
                </button>
              )}
            </div>
          )}

          {uploadState.error && (
            <div className="flex items-center gap-2 rounded-md bg-red-900/20 border border-red-900/50 p-3 text-red-400">
              <AlertCircle className="h-4 w-4" />
              <span className="text-sm">{uploadState.error}</span>
            </div>
          )}

          {uploadState.success && uploadState.receipt && (
            <div className="flex items-center gap-2 rounded-md bg-green-900/20 border border-green-900/50 p-3 text-green-400">
              <CheckCircle className="h-4 w-4" />
              <span className="text-sm">File uploaded successfully!</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}