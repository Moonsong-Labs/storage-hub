'use client';

import { useState, useRef, useCallback } from 'react';
import { Upload, Download, File, Folder, Hash, X, CheckCircle, AlertCircle, Plus, Database, ArrowLeft } from 'lucide-react';
import { type WalletClient, type PublicClient } from 'viem';
import { FileManager as StorageHubFileManager, initWasm, StorageHubClient, ReplicationLevel } from '@storagehub-sdk/core';
import { MspClient, type UploadReceipt, type Bucket, type FileTree } from '@storagehub-sdk/msp-client';
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

interface UploadLocationState {
  selectedPath: string;
  isNavigating: boolean;
  availableFolders: FileTree[];
  navigationHistory: string[];
  showFolderCreator: boolean;
  newFolderName: string;
  isLoadingFolders: boolean;
}

interface BucketCreationState {
  bucketName: string;
  isCreating: boolean;
  error: string | null;
  success: boolean;
  createdBucketId: string | null;
}

interface FileBrowserState {
  selectedBucketId: string | null;
  currentPath: string;
  files: FileTree[];
  isLoading: boolean;
  error: string | null;
  selectedFile: FileTree | null;
}

interface FileDownloadState {
  downloadingFiles: Set<string>; // Track which files are being downloaded by fileKey
  downloadError: string | null;
}

export function FileManager({ publicClient, walletAddress, mspClient, storageHubClient }: FileManagerProps) {
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

  const [uploadLocationState, setUploadLocationState] = useState<UploadLocationState>({
    selectedPath: '',
    isNavigating: false,
    availableFolders: [],
    navigationHistory: [],
    showFolderCreator: false,
    newFolderName: '',
    isLoadingFolders: false
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
  const [isLoadingBuckets, setIsLoadingBuckets] = useState<boolean>(false);

  // File Browser State
  const [fileBrowserState, setFileBrowserState] = useState<FileBrowserState>({
    selectedBucketId: null,
    currentPath: '',
    files: [],
    isLoading: false,
    error: null,
    selectedFile: null,
  });

  // File Download State
  const [downloadState, setDownloadState] = useState<FileDownloadState>({
    downloadingFiles: new Set(),
    downloadError: null,
  });

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
      // Derive MSP info dynamically via MSP client
      if (!mspClient) throw new Error('MSP client not connected');

      const info = await mspClient.info.getInfo();
      const valueProps = await mspClient.info.getValuePropositions();
      const mspId = (info.mspId || '') as `0x${string}`;
      const valuePropId = (valueProps.find(v => v.isAvailable)?.id || valueProps[0]?.id || '') as `0x${string}`;
      if (!mspId || !valuePropId) throw new Error('Missing MSP identifiers');

      const bucketId = await storageHubClient.deriveBucketId(walletAddress as `0x${string}`, bucketState.bucketName);

      const txHash = await storageHubClient.createBucket(
        mspId,
        bucketState.bucketName,
        false,
        valuePropId,
        undefined
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


        // Refresh bucket list from MSP backend to get the latest state
        await loadBuckets();
      } else {
        throw new Error('Bucket creation transaction failed');
      }
    } catch (error: unknown) {
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

      let bucketList: Bucket[] = [];
      try {
        bucketList = await mspClient.buckets.listBuckets();
      } catch (sdkError: unknown) {
        console.error('❌ Failed to load buckets:', sdkError instanceof Error ? sdkError.message : sdkError);
        bucketList = []; // Fallback to empty array
      }

      // Replace all buckets with the fresh list from MSP backend
      const freshBuckets = bucketList || [];
      setBuckets(freshBuckets);

    } catch (error: unknown) {
      console.error('❌ Failed to refresh buckets:', error instanceof Error ? error.message : error);
    } finally {
      setIsLoadingBuckets(false);
    }
  };

  // Note: loadBuckets is only called manually via refresh button or after bucket creation
  // No automatic loading to avoid excessive API calls

  // Load files from selected bucket
  const loadFiles = async (bucketId: string, path: string = '') => {
    if (!mspClient) {
      console.warn('⚠️ MSP client not available, cannot load files');
      return;
    }

    setFileBrowserState(prev => ({ ...prev, isLoading: true, error: null }));

    try {

      const fileListResponse = await mspClient.buckets.getFiles(bucketId, path ? { path } : undefined);
      console.log('📦 fileListResponse:', fileListResponse);

      // Extract files from the hierarchical tree structure
      let extractedFiles: FileTree[] = [];

      if (fileListResponse?.files && fileListResponse.files.length > 0) {
        const rootTree = fileListResponse.files[0]; // First element is the root folder

        // The API returns a single FileTree with flattened structure
        if (rootTree?.type === 'folder' && rootTree.children) {
          // Extract direct children from the root folder
          extractedFiles = rootTree.children;
        } else {
          // Fallback: treat as flat FileTree array
          extractedFiles = fileListResponse.files as FileTree[];
        }
      }

      setFileBrowserState(prev => ({
        ...prev,
        selectedBucketId: bucketId,
        currentPath: path,
        files: extractedFiles,
        isLoading: false,
        error: null,
      }));

      console.log(`📁 Loaded ${extractedFiles.length} files`);

    } catch (error: unknown) {
      console.error('❌ Failed to load files:', error);
      setFileBrowserState(prev => ({
        ...prev,
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to load files',
      }));
    }
  };

  // File upload function
  const uploadFile = async () => {
    if (!uploadState.file || !uploadState.fingerprint || !mspClient || !storageHubClient || !walletAddress || !selectedBucketId) return;

    setUploadState(prev => ({ ...prev, isUploading: true, error: null }));

    try {
      await initWasm();

      // Use the selected upload path or default to root
      const basePath = uploadLocationState.selectedPath || '';
      const fileLocation = basePath
        ? `${basePath}/${uploadState.file.name}`
        : uploadState.file.name;

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

      if (fileSizeNumber === undefined || fileSizeNumber === null) {
        throw new Error(`FileManager.getFileSize() returned ${fileSizeNumber}`);
      }

      const fileSize = BigInt(fileSizeNumber);

      // Create TypeRegistry and types for file key computation (like sdk-precompiles)
      const registry = new TypeRegistry();
      const owner = registry.createType("AccountId20", walletAddress) as AccountId20;


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
      // File key is computed by the MSP backend during upload
      await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      setUploadState(prev => ({ ...prev, uploadProgress: 25 }));

      // Issue storage request
      // Derive IDs from MSP client
      const info = await mspClient.info.getInfo();
      const valueProps = await mspClient.info.getValuePropositions();
      const mspId = info.mspId as `0x${string}`;
      let mspPeerId = '';
      if (Array.isArray(info.multiaddresses)) {
        for (const ma of info.multiaddresses) {
          const idx = ma.lastIndexOf('/p2p/');
          if (idx !== -1) {
            mspPeerId = ma.slice(idx + 5);
            break;
          }
        }
        if (!mspPeerId && info.multiaddresses.length > 0) {
          const first = info.multiaddresses[0];
          const idx = first.lastIndexOf('/p2p/');
          mspPeerId = idx !== -1 ? first.slice(idx + 5) : first;
        }
      }
      const valuePropId = (valueProps.find(v => v.isAvailable)?.id || valueProps[0]?.id || '') as `0x${string}`;


      // Ensure bucket ID has 0x prefix for storage request
      const bucketIdForStorageRequest = selectedBucketId.startsWith('0x') ? selectedBucketId : `0x${selectedBucketId}`;

      let storageRequestTxHash;
      try {

        storageRequestTxHash = await storageHubClient.issueStorageRequest(
          bucketIdForStorageRequest as `0x${string}`,
          fileLocation,
          fingerprint.toHex() as `0x${string}`, // Use hex string like sdk-precompiles
          fileSize,
          mspId,
          mspPeerId ? [mspPeerId] : [],
          ReplicationLevel.Basic,
          0
          // No gas options - let it estimate naturally like sdk-precompiles
        );
      } catch (error: unknown) {
        console.error('❌ Storage request failed:', error instanceof Error ? error.message : error);
        throw error;
      }

      const storageRequestReceipt = await publicClient!.waitForTransactionReceipt({
        hash: storageRequestTxHash
      });

      if (storageRequestReceipt.status !== 'success') {
        throw new Error('Storage request transaction failed');
      }

      setUploadState(prev => ({ ...prev, uploadProgress: 30 }));

      // CRITICAL: Recompute file key AFTER storage request (like sdk-precompiles line 215)
      const finalFileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      // Wait a moment for MSP to process the storage request (like sdk-precompiles)
      await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2 seconds
      setUploadState(prev => ({ ...prev, uploadProgress: 40 }));

      let uploadReceipt;
      try {
        // Upload file to MSP (use exact same pattern as sdk-precompiles line 245-251)
        const fileBlob = await fileManager.getFileBlob(); // Get Blob like sdk-precompiles
        const fileKeyHex = finalFileKey.toHex();

        await new Promise(resolve => setTimeout(resolve, 3000)); // Add a 3 second delay before uploading

        uploadReceipt = await mspClient.files.uploadFile(
          selectedBucketId, // MSP expects bucket ID without 0x prefix
          fileKeyHex, // Use the final computed file key
          fileBlob, // Use Blob instead of File object
          walletAddress, // owner parameter like sdk-precompiles
          fileLocation // location parameter like sdk-precompiles
        );

      } catch (error: unknown) {
        console.error('❌ MSP upload failed:', error instanceof Error ? error.message : error);
        throw error;
      }

      setUploadState(prev => ({
        ...prev,
        isUploading: false,
        success: true,
        receipt: uploadReceipt,
        error: null,
        uploadProgress: 100
      }));

    } catch (error: unknown) {
      console.error('Upload failed:', error);
      setUploadState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Upload failed',
        isUploading: false
      }));
    }
  };

  // Folder navigation functions
  const openFolderBrowser = async () => {
    if (!selectedBucketId) {
      alert('Please select a bucket first');
      return;
    }

    setUploadLocationState(prev => ({ ...prev, isNavigating: true, isLoadingFolders: true }));

    try {
      // Load folders from the current selected path, not always from root
      if (!mspClient) {
        console.error('MSP client not available');
        setUploadLocationState(prev => ({ ...prev, isNavigating: false, isLoadingFolders: false }));
        return;
      }

      const currentPath = uploadLocationState.selectedPath || '';
      console.log('Opening folder browser for current path:', currentPath);

      const fileListResponse = await mspClient.buckets.getFiles(selectedBucketId, currentPath ? { path: currentPath } : undefined);
      console.log('API response for current path', currentPath, ':', fileListResponse);

      if (fileListResponse?.files && fileListResponse.files.length > 0) {
        const folderTree = fileListResponse.files[0];
        console.log('Folder tree for current path', currentPath, ':', folderTree);

        if (folderTree?.type === 'folder' && folderTree.children) {
          const subfolders = folderTree.children.filter(child => child.type === 'folder');
          console.log('Found subfolders in current path', currentPath, ':', subfolders);

          setUploadLocationState(prev => ({
            ...prev,
            availableFolders: subfolders,
            navigationHistory: currentPath ? [currentPath] : ['/'],
            isLoadingFolders: false
          }));
        } else {
          console.log('No subfolders found in current path', currentPath);
          setUploadLocationState(prev => ({
            ...prev,
            availableFolders: [],
            navigationHistory: currentPath ? [currentPath] : ['/'],
            isLoadingFolders: false
          }));
        }
      } else {
        console.log('No files found in current path', currentPath);
        setUploadLocationState(prev => ({
          ...prev,
          availableFolders: [],
          navigationHistory: currentPath ? [currentPath] : ['/'],
          isLoadingFolders: false
        }));
      }
    } catch (error) {
      console.error('Failed to load folders:', error);
      setUploadLocationState(prev => ({ ...prev, isNavigating: false, isLoadingFolders: false }));
    }
  };

  const navigateToFolder = async (folderName: string) => {
    if (!mspClient || !selectedBucketId) {
      console.error('MSP client or bucket not available');
      return;
    }

    const newPath = uploadLocationState.selectedPath
      ? `${uploadLocationState.selectedPath}/${folderName}`
      : folderName;

    setUploadLocationState(prev => ({ ...prev, isLoadingFolders: true }));

    try {
      // Load the contents of the selected folder
      console.log('Navigating to folder:', folderName, 'with path:', newPath);
      const fileListResponse = await mspClient.buckets.getFiles(selectedBucketId, { path: newPath });
      console.log('API response for path', newPath, ':', fileListResponse);

      if (fileListResponse?.files && fileListResponse.files.length > 0) {
        const folderTree = fileListResponse.files[0];
        console.log('Folder tree for path', newPath, ':', folderTree);

        if (folderTree?.type === 'folder' && folderTree.children) {
          const subfolders = folderTree.children.filter(child => child.type === 'folder');
          console.log('Found subfolders in', newPath, ':', subfolders);

          setUploadLocationState(prev => ({
            ...prev,
            selectedPath: newPath,
            navigationHistory: [...prev.navigationHistory, newPath],
            availableFolders: subfolders,
            isLoadingFolders: false
          }));
        } else {
          // If it's not a folder or has no children, show empty state
          console.log('No subfolders found in', newPath);
          setUploadLocationState(prev => ({
            ...prev,
            selectedPath: newPath,
            navigationHistory: [...prev.navigationHistory, newPath],
            availableFolders: [],
            isLoadingFolders: false
          }));
        }
      } else {
        // No files found in this folder
        console.log('No files found in path', newPath);
        setUploadLocationState(prev => ({
          ...prev,
          selectedPath: newPath,
          navigationHistory: [...prev.navigationHistory, newPath],
          availableFolders: [],
          isLoadingFolders: false
        }));
      }
    } catch (error) {
      console.error('Failed to load folder contents:', error);
      // Still update the path even if loading fails
      setUploadLocationState(prev => ({
        ...prev,
        selectedPath: newPath,
        navigationHistory: [...prev.navigationHistory, newPath],
        availableFolders: [],
        isLoadingFolders: false
      }));
    }
  };

  const navigateBack = async () => {
    if (uploadLocationState.navigationHistory.length > 1) {
      const newHistory = uploadLocationState.navigationHistory.slice(0, -1);
      const newPath = newHistory[newHistory.length - 1] || '';

      if (!mspClient || !selectedBucketId) {
        console.error('MSP client or bucket not available');
        return;
      }

      setUploadLocationState(prev => ({ ...prev, isLoadingFolders: true }));

      try {
        // Load the contents of the parent folder
        const fileListResponse = await mspClient.buckets.getFiles(selectedBucketId, { path: newPath });

        if (fileListResponse?.files && fileListResponse.files.length > 0) {
          const folderTree = fileListResponse.files[0];
          if (folderTree?.type === 'folder' && folderTree.children) {
            setUploadLocationState(prev => ({
              ...prev,
              selectedPath: newPath,
              navigationHistory: newHistory,
              availableFolders: folderTree.children.filter(child => child.type === 'folder')
            }));
          } else {
            setUploadLocationState(prev => ({
              ...prev,
              selectedPath: newPath,
              navigationHistory: newHistory,
              availableFolders: []
            }));
          }
        } else {
          setUploadLocationState(prev => ({
            ...prev,
            selectedPath: newPath,
            navigationHistory: newHistory,
            availableFolders: []
          }));
        }
      } catch (error) {
        console.error('Failed to load parent folder contents:', error);
        setUploadLocationState(prev => ({
          ...prev,
          selectedPath: newPath,
          navigationHistory: newHistory,
          availableFolders: []
        }));
      }
    }
  };

  const selectCurrentPath = () => {
    setUploadLocationState(prev => ({ ...prev, isNavigating: false }));
  };

  const createNewFolder = async () => {
    if (!uploadLocationState.newFolderName.trim()) return;

    // For now, we'll just add it to the path - in a real implementation,
    // you might want to create the folder on the server
    const newPath = uploadLocationState.selectedPath
      ? `${uploadLocationState.selectedPath}/${uploadLocationState.newFolderName.trim()}`
      : uploadLocationState.newFolderName.trim();

    setUploadLocationState(prev => ({
      ...prev,
      selectedPath: newPath,
      showFolderCreator: false,
      newFolderName: ''
    }));
  };

  const resetToRoot = async () => {
    if (!mspClient || !selectedBucketId) {
      console.error('MSP client or bucket not available');
      return;
    }

    try {
      // Load the root folder contents
      const fileListResponse = await mspClient.buckets.getFiles(selectedBucketId);

      if (fileListResponse?.files && fileListResponse.files.length > 0) {
        const rootTree = fileListResponse.files[0];
        if (rootTree?.type === 'folder' && rootTree.children) {
          setUploadLocationState(prev => ({
            ...prev,
            selectedPath: '',
            navigationHistory: ['/'],
            availableFolders: rootTree.children.filter(child => child.type === 'folder')
          }));
        } else {
          setUploadLocationState(prev => ({
            ...prev,
            selectedPath: '',
            navigationHistory: ['/'],
            availableFolders: []
          }));
        }
      } else {
        setUploadLocationState(prev => ({
          ...prev,
          selectedPath: '',
          navigationHistory: ['/'],
          availableFolders: []
        }));
      }
    } catch (error) {
      console.error('Failed to load root folder contents:', error);
      setUploadLocationState(prev => ({
        ...prev,
        selectedPath: '',
        navigationHistory: ['/'],
        availableFolders: []
      }));
    }
  };

  // File download function
  const downloadFile = async (file: FileTree) => {
    if (!mspClient || !file.fileKey) {
      console.error('Cannot download: missing MSP client or file key');
      return;
    }

    const fileKey = file.fileKey;
    console.log('🔄 Starting download for file:', file.name, 'with key:', fileKey);

    // Add file to downloading set
    setDownloadState(prev => ({
      ...prev,
      downloadingFiles: new Set([...prev.downloadingFiles, fileKey]),
      downloadError: null
    }));

    try {
      // Download file using MSP SDK
      console.log('📥 Calling mspClient.files.downloadFile...');
      const downloadResult = await mspClient.files.downloadFile(fileKey);

      console.log('✅ Download response received:', {
        status: downloadResult.status,
        contentType: downloadResult.contentType,
        contentLength: downloadResult.contentLength
      });

      if (downloadResult.status !== 200) {
        throw new Error(`Download failed with status: ${downloadResult.status}`);
      }

      // Convert stream to blob
      console.log('🔄 Converting stream to blob...');
      const reader = downloadResult.stream.getReader();
      const chunks: Uint8Array[] = [];

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      // Calculate total length and create combined array
      const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
      const combinedArray = new Uint8Array(totalLength);
      let offset = 0;

      for (const chunk of chunks) {
        combinedArray.set(chunk, offset);
        offset += chunk.length;
      }

      // Create blob and download URL
      const blob = new Blob([combinedArray], {
        type: downloadResult.contentType || 'application/octet-stream'
      });

      console.log('📁 Created blob:', {
        size: blob.size,
        type: blob.type
      });

      // Create download link and trigger download
      const downloadUrl = URL.createObjectURL(blob);
      const downloadLink = document.createElement('a');
      downloadLink.href = downloadUrl;
      downloadLink.download = file.name;

      // Append to body, click, and remove
      document.body.appendChild(downloadLink);
      downloadLink.click();
      document.body.removeChild(downloadLink);

      // Clean up the URL object
      URL.revokeObjectURL(downloadUrl);

      console.log('✅ File download completed:', file.name);

    } catch (error: unknown) {
      console.error('❌ Download failed:', error);
      setDownloadState(prev => ({
        ...prev,
        downloadError: error instanceof Error ? error.message : 'Download failed'
      }));
    } finally {
      // Remove file from downloading set
      setDownloadState(prev => ({
        ...prev,
        downloadingFiles: new Set([...prev.downloadingFiles].filter(key => key !== fileKey))
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
              {buckets.map((bucket) => (
                <option key={bucket.bucketId} value={bucket.bucketId}>
                  {bucket.name} ({bucket.bucketId.slice(0, 8)}...)
                </option>
              ))}
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

              {/* Upload Location Selector */}
              {uploadState.fingerprint && (
                <div className="space-y-3">
                  <div className="flex items-center gap-2">
                    <Folder className="h-4 w-4 text-blue-400" />
                    <span className="text-sm font-medium">Upload Location</span>
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-gray-400">Path:</span>
                      <span className="text-xs font-mono text-gray-300">
                        {uploadLocationState.selectedPath || '/'}
                      </span>
                    </div>

                    <div className="flex gap-2">
                      <button
                        onClick={openFolderBrowser}
                        className="flex items-center gap-1 px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                      >
                        <Folder className="h-3 w-3" />
                        Browse Folders
                      </button>

                      <button
                        onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: true }))}
                        className="flex items-center gap-1 px-3 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
                      >
                        <Plus className="h-3 w-3" />
                        Create Folder
                      </button>

                      <button
                        onClick={resetToRoot}
                        className="flex items-center gap-1 px-3 py-1 text-xs bg-gray-600 text-white rounded hover:bg-gray-700 transition-colors"
                      >
                        <Hash className="h-3 w-3" />
                        Root
                      </button>
                    </div>
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

        {/* File Browser Section */}
        <div className="mt-8 space-y-4">
          <div className="flex items-center gap-2">
            <Folder className="h-5 w-5 text-blue-400" />
            <h3 className="text-lg font-medium">Browse Files</h3>
          </div>

          {/* Browser Controls */}
          <div className="flex gap-4 items-center">
            <select
              value={fileBrowserState.selectedBucketId || ''}
              onChange={(e) => {
                const bucketId = e.target.value;
                if (bucketId) {
                  loadFiles(bucketId);
                }
              }}
              className="flex-1 rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 focus:outline-none"
            >
              <option value="">Select bucket to browse...</option>
              {buckets.map((bucket) => (
                <option key={bucket.bucketId} value={bucket.bucketId}>
                  {bucket.name} ({bucket.fileCount} files)
                </option>
              ))}
            </select>

            {fileBrowserState.selectedBucketId && (
              <>
                <button
                  onClick={() => loadFiles(fileBrowserState.selectedBucketId!, fileBrowserState.currentPath)}
                  disabled={fileBrowserState.isLoading}
                  className="px-4 py-2 text-sm bg-gray-700 text-gray-300 rounded-md hover:bg-gray-600 disabled:bg-gray-800 disabled:cursor-not-allowed transition-colors"
                >
                  {fileBrowserState.isLoading ? 'Loading...' : 'Refresh'}
                </button>

                {/* Back Button - only show if we're not at root */}
                {fileBrowserState.currentPath && fileBrowserState.currentPath !== '/' && (
                  <button
                    onClick={() => {
                      // Navigate back one level
                      const pathParts = fileBrowserState.currentPath.split('/');
                      const parentPath = pathParts.slice(0, -1).join('/') || '';
                      loadFiles(fileBrowserState.selectedBucketId!, parentPath);
                    }}
                    disabled={fileBrowserState.isLoading}
                    className="flex items-center gap-1 px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:bg-gray-800 disabled:cursor-not-allowed transition-colors"
                  >
                    <ArrowLeft className="h-4 w-4" />
                    Back
                  </button>
                )}
              </>
            )}
          </div>

          {/* Path Breadcrumb */}
          {fileBrowserState.selectedBucketId && (
            <div className="flex items-center gap-2 text-sm text-gray-400">
              <Folder className="h-4 w-4" />
              <span>
                {buckets.find(b => b.bucketId === fileBrowserState.selectedBucketId)?.name || 'Unknown Bucket'}
              </span>
              {fileBrowserState.currentPath && (
                <>
                  <span>/</span>
                  <span>{fileBrowserState.currentPath}</span>
                </>
              )}
            </div>
          )}

          {/* File List */}
          {fileBrowserState.selectedBucketId && (
            <div className="border border-gray-700 rounded-lg overflow-hidden">
              {fileBrowserState.isLoading ? (
                <div className="p-8 text-center text-gray-500">
                  <div className="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-2"></div>
                  Loading files...
                </div>
              ) : fileBrowserState.error ? (
                <div className="p-4 bg-red-900/20 border-red-900/50 text-red-400 text-sm">
                  <AlertCircle className="h-4 w-4 inline mr-2" />
                  {fileBrowserState.error}
                </div>
              ) : fileBrowserState.files.length === 0 ? (
                <div className="p-8 text-center text-gray-500">
                  <Folder className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>No files found in this bucket</p>
                  <p className="text-sm mt-1">Upload some files to see them here</p>
                </div>
              ) : (
                <div className="divide-y divide-gray-700">
                  {fileBrowserState.files.map((file, index) => (
                    <div
                      key={`${file.name}-${index}`}
                      className={`p-4 hover:bg-gray-800 cursor-pointer transition-colors ${fileBrowserState.selectedFile === file ? 'bg-blue-900/20 border-l-4 border-blue-500' : ''
                        }`}
                      onClick={() => setFileBrowserState(prev => ({
                        ...prev,
                        selectedFile: prev.selectedFile === file ? null : file
                      }))}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          {file.type === 'folder' ? (
                            <Folder className="h-5 w-5 text-blue-400" />
                          ) : (
                            <File className="h-5 w-5 text-gray-400" />
                          )}
                          <div>
                            <div className="text-sm font-medium text-gray-200">{file.name}</div>
                            <div className="text-xs text-gray-500">
                              {file.type === 'file' ? (
                                <>
                                  {file.sizeBytes ? `${(file.sizeBytes / 1024).toFixed(1)} KB` : 'Unknown size'}
                                  {file.fileKey && (
                                    <span className="ml-2">• Key: {file.fileKey.slice(0, 8)}...</span>
                                  )}
                                </>
                              ) : (
                                'Folder'
                              )}
                            </div>
                          </div>
                        </div>

                        <div className="flex items-center gap-2">
                          {file.type === 'file' && file.fileKey && (
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                downloadFile(file);
                              }}
                              disabled={downloadState.downloadingFiles.has(file.fileKey || '')}
                              className="px-3 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors"
                            >
                              {downloadState.downloadingFiles.has(file.fileKey || '') ? (
                                <>
                                  <div className="animate-spin h-3 w-3 border border-white border-t-transparent rounded-full inline mr-1"></div>
                                  Downloading...
                                </>
                              ) : (
                                <>
                                  <Download className="h-3 w-3 inline mr-1" />
                                  Download
                                </>
                              )}
                            </button>
                          )}
                          {file.type === 'folder' && (
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                const newPath = fileBrowserState.currentPath
                                  ? `${fileBrowserState.currentPath}/${file.name}`
                                  : file.name;
                                loadFiles(fileBrowserState.selectedBucketId!, newPath);
                              }}
                              className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                            >
                              Open
                            </button>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Download Error */}
          {downloadState.downloadError && (
            <div className="p-4 bg-red-900/20 border border-red-900/50 rounded-lg">
              <div className="flex items-center gap-2 text-red-400">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm font-medium">Download Failed</span>
                <button
                  onClick={() => setDownloadState(prev => ({ ...prev, downloadError: null }))}
                  className="ml-auto text-red-400 hover:text-red-300"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
              <p className="text-sm text-red-300 mt-2">{downloadState.downloadError}</p>
            </div>
          )}

          {/* Selected File Info */}
          {fileBrowserState.selectedFile && (
            <div className="p-4 bg-gray-800 rounded-lg border border-gray-700">
              <h4 className="text-sm font-medium text-gray-200 mb-2">File Information</h4>
              <div className="space-y-1 text-xs text-gray-400">
                <div><strong>Name:</strong> {fileBrowserState.selectedFile.name}</div>
                <div><strong>Type:</strong> {fileBrowserState.selectedFile.type}</div>
                {fileBrowserState.selectedFile.sizeBytes && (
                  <div><strong>Size:</strong> {(fileBrowserState.selectedFile.sizeBytes / 1024).toFixed(2)} KB</div>
                )}
                {fileBrowserState.selectedFile.fileKey && (
                  <div><strong>File Key:</strong> {fileBrowserState.selectedFile.fileKey}</div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Folder Browser Modal */}
      {uploadLocationState.isNavigating && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-900 border border-gray-700 rounded-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-hidden">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-200">Select Upload Location</h3>
              <button
                onClick={() => {
                  setUploadLocationState(prev => ({
                    ...prev,
                    isNavigating: false,
                    selectedPath: '', // Reset to root
                    navigationHistory: ['/']
                  }));
                }}
                className="text-gray-400 hover:text-white"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            {/* Breadcrumb Navigation */}
            <div className="flex items-center gap-2 mb-4 p-2 bg-gray-800 rounded">
              <button
                onClick={resetToRoot}
                className="flex items-center gap-1 px-2 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                <Hash className="h-3 w-3" />
                Root
              </button>

              {uploadLocationState.navigationHistory.length > 1 && (
                <button
                  onClick={navigateBack}
                  className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-600 text-white rounded hover:bg-gray-700"
                >
                  ← Back
                </button>
              )}

              <span className="text-sm text-gray-400">
                Current Path: {uploadLocationState.selectedPath || '/'}
              </span>

              <span className="text-xs text-gray-500">
                (Folders: {uploadLocationState.availableFolders.length})
              </span>
            </div>

            {/* Folder List */}
            <div className="space-y-2 mb-4 max-h-60 overflow-y-auto">
              {uploadLocationState.isLoadingFolders ? (
                <div className="text-center py-8 text-gray-500">
                  <div className="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-2"></div>
                  <p>Loading folders...</p>
                </div>
              ) : uploadLocationState.availableFolders.length === 0 ? (
                <div className="text-center py-8 text-gray-500">
                  <Folder className="h-12 w-12 mx-auto mb-2 opacity-50" />
                  <p>No folders found</p>
                </div>
              ) : (
                uploadLocationState.availableFolders.map((folder, index) => (
                  <div
                    key={`${folder.name}-${index}`}
                    onClick={() => navigateToFolder(folder.name)}
                    className="flex items-center gap-3 p-3 bg-gray-800 rounded hover:bg-gray-700 cursor-pointer transition-colors"
                  >
                    <Folder className="h-5 w-5 text-blue-400" />
                    <span className="text-sm text-gray-200">{folder.name}</span>
                    <span className="text-xs text-gray-500 ml-auto">→</span>
                    <span className="text-xs text-gray-600 ml-2">
                      (in {uploadLocationState.selectedPath || 'root'})
                    </span>
                  </div>
                ))
              )}
            </div>

            {/* Action Buttons */}
            <div className="flex gap-2">
              <button
                onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: true }))}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-700"
              >
                <Plus className="h-4 w-4" />
                Create New Folder
              </button>

              <button
                onClick={selectCurrentPath}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                <CheckCircle className="h-4 w-4" />
                Select This Location
              </button>

              <button
                onClick={() => {
                  setUploadLocationState(prev => ({
                    ...prev,
                    isNavigating: false,
                    selectedPath: '', // Reset to root
                    navigationHistory: ['/']
                  }));
                }}
                className="px-4 py-2 text-sm bg-gray-600 text-white rounded hover:bg-gray-700"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Folder Creator Modal */}
      {uploadLocationState.showFolderCreator && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-900 border border-gray-700 rounded-lg p-6 w-full max-w-md">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-200">Create New Folder</h3>
              <button
                onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: false, newFolderName: '' }))}
                className="text-gray-400 hover:text-white"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Folder Name
                </label>
                <input
                  type="text"
                  value={uploadLocationState.newFolderName}
                  onChange={(e) => setUploadLocationState(prev => ({ ...prev, newFolderName: e.target.value }))}
                  placeholder="Enter folder name"
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-gray-100 focus:border-blue-500 focus:outline-none"
                  autoFocus
                />
              </div>

              <div className="text-sm text-gray-400">
                <span>Will be created in: </span>
                <span className="font-mono">{uploadLocationState.selectedPath || '/'}</span>
              </div>

              <div className="flex gap-2">
                <button
                  onClick={createNewFolder}
                  disabled={!uploadLocationState.newFolderName.trim()}
                  className="flex-1 px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
                >
                  Create Folder
                </button>

                <button
                  onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: false, newFolderName: '' }))}
                  className="px-4 py-2 text-sm bg-gray-600 text-white rounded hover:bg-gray-700"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}