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

      const bucketId = await storageHubClient.deriveBucketId(walletAddress, bucketState.bucketName);
      console.log('Derived bucket ID:', bucketId);

      const txHash = await storageHubClient.createBucket(
        TEST_MSP_ID,
        bucketState.bucketName,
        false, // isPrivate
        TEST_VALUE_PROP_ID,
        {
          // Explicit gas settings to avoid estimation issues
          gas: 500_000n, // Explicit gas limit
          gasPrice: BigInt('1000000000') // 1 gwei
        }
      );

      console.log('Bucket creation transaction submitted:', txHash);

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });

      if (receipt.status === 'success') {
        setBucketState(prev => ({
          ...prev,
          isCreating: false,
          success: true,
          createdBucketId: bucketId as string,
          error: null
        }));

        // Add the created bucket to local state immediately
        const newBucket: Bucket = {
          bucketId: bucketId as string,
          name: bucketState.bucketName,
          root: '/',
          isPublic: true, // We created it as public (isPrivate: false)
          sizeBytes: 0,
          valuePropId: TEST_VALUE_PROP_ID,
          fileCount: 0,
        };
        
        setBuckets(prev => {
          const updatedBuckets = [...prev, newBucket];
          console.log('✅ Added bucket to local state:', newBucket);
          console.log('📋 All buckets now:', updatedBuckets);
          return updatedBuckets;
        });

        console.log('🎉 Bucket creation completed successfully!');
      } else {
        throw new Error('Bucket creation transaction failed');
      }
    } catch (error) {
      console.error('Bucket creation failed:', error);
      setBucketState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Bucket creation failed',
        isCreating: false
      }));
    }
  };

  // Load buckets
  const loadBuckets = async () => {
    if (!mspClient) return;
    
    try {
      console.log('🔍 Loading buckets from MSP...');
      const bucketList = await mspClient.listBuckets();
      console.log('✅ Buckets loaded from MSP:', bucketList);
      
      // Merge MSP buckets with existing local buckets, avoiding duplicates
      setBuckets(prev => {
        const mspBuckets = bucketList || [];
        const existingIds = prev.map(b => b.bucketId);
        const newMspBuckets = mspBuckets.filter(b => !existingIds.includes(b.bucketId));
        const merged = [...prev, ...newMspBuckets];
        console.log('🔄 Merged buckets (local + MSP):', merged);
        return merged;
      });
    } catch (error) {
      console.error('❌ Failed to load buckets from MSP:', error);
      // Don't clear existing buckets if MSP fails
      console.log('🔄 Keeping existing local buckets due to MSP error');
    }
  };

  // Load buckets when component mounts and when MSP client changes
  useEffect(() => {
    if (mspClient) {
      loadBuckets();
    }
  }, [mspClient]); // eslint-disable-line react-hooks/exhaustive-deps

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
      const bucketIdH256 = registry.createType("H256", selectedBucketId) as H256;
      const fileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      console.log('📋 File metadata computed:', {
        owner: walletAddress,
        bucketId: selectedBucketId,
        location: fileLocation,
        fingerprint: fingerprint.toHex(),
        fileKey: fileKey.toHex(),
        fileSize: fileSize.toString()
      });

      setUploadState(prev => ({ ...prev, uploadProgress: 25 }));

      // Issue storage request
      const TEST_MSP_ID = '0x0000000000000000000000000000000000000000000000000000000000000300';
      const MSP_PEER_ID = 'coolMSPWMvbhtYjbhgjoDzbnf71SFznJAKBBkSGYEUtnpES1y9tM';

      // EXTENSIVE DEBUGGING - Check every single parameter
      console.log('🔍 DEBUGGING ALL PARAMETERS:');
      console.log('selectedBucketId:', selectedBucketId, typeof selectedBucketId);
      console.log('fileLocation:', fileLocation, typeof fileLocation);
      console.log('fingerprint object:', fingerprint);
      console.log('fingerprint.toHex():', fingerprint.toHex(), typeof fingerprint.toHex());
      console.log('fileSize BigInt:', fileSize, typeof fileSize);
      console.log('TEST_MSP_ID:', TEST_MSP_ID, typeof TEST_MSP_ID);
      console.log('MSP_PEER_ID:', MSP_PEER_ID, typeof MSP_PEER_ID);
      console.log('ReplicationLevel.Basic:', ReplicationLevel.Basic, typeof ReplicationLevel.Basic);
      
      // Check if any are undefined
      const params = [
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
        storageRequestTxHash = await storageHubClient.issueStorageRequest(
          selectedBucketId as `0x${string}`,
          fileLocation,
          fingerprint.toHex() as `0x${string}`, // Use hex string like sdk-precompiles
          fileSize,
          TEST_MSP_ID as `0x${string}`,
          [MSP_PEER_ID],
          ReplicationLevel.Basic,
          0 // replicas (used only when ReplicationLevel = Custom, like sdk-precompiles)
          // No gas options - let it estimate naturally like sdk-precompiles
        );
        
        console.log('✅ Storage request submitted successfully:', storageRequestTxHash);
      } catch (error) {
        console.error('❌ DETAILED ERROR in issueStorageRequest:');
        console.error('Error message:', error.message);
        console.error('Error stack:', error.stack);
        console.error('Full error object:', error);
        throw error; // Re-throw to maintain the original behavior
      }

      const storageRequestReceipt = await publicClient.waitForTransactionReceipt({ 
        hash: storageRequestTxHash 
      });

      if (storageRequestReceipt.status !== 'success') {
        throw new Error('Storage request transaction failed');
      }

      setUploadState(prev => ({ ...prev, uploadProgress: 50 }));

      // Upload file to MSP (use exact same pattern as sdk-precompiles)
      const fileBlob = await fileManager.getFileBlob(); // Get Blob like sdk-precompiles
      const uploadReceipt = await mspClient.uploadFile(
        selectedBucketId,
        fileKey.toHex(), // Convert H256 to hex string
        fileBlob, // Use Blob instead of File object
        walletAddress, // owner parameter like sdk-precompiles
        fileLocation // location parameter like sdk-precompiles
      );

      setUploadState(prev => ({
        ...prev,
        isUploading: false,
        success: true,
        receipt: uploadReceipt,
        uploadProgress: 100
      }));

    } catch (error) {
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
            {buckets.length > 0 && (
              <span className="text-xs text-gray-500 ml-2">
                [{buckets.map(b => b.name).join(', ')}]
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
                return (
                  <option key={bucket.bucketId} value={bucket.bucketId}>
                    {bucket.name} ({bucket.bucketId.slice(0, 8)}...)
                  </option>
                );
              })}
            </select>
            <button
              onClick={loadBuckets}
              className="px-4 py-2 text-sm bg-gray-700 text-gray-300 rounded-md hover:bg-gray-600"
            >
              Refresh
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