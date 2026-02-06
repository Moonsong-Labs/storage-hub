'use client';

import { useState, useRef, useCallback, useId } from 'react';
import { Upload, Download, File, Folder, Hash, X, CheckCircle, AlertCircle, Plus, Database, ArrowLeft, Trash2 } from 'lucide-react';
import type { WalletClient, PublicClient } from 'viem';
import {
  decryptFile,
  encryptFile,
  FileManager as StorageHubFileManager,
  generateEncryptionKey,
  IKM,
  initWasm,
  readEncryptionHeader,
  type StorageHubClient,
  ReplicationLevel,
  type FileInfo as CoreFileInfo,
} from '@storagehub-sdk/core';
import type { MspClient } from '@storagehub-sdk/msp-client';
import type { UploadReceipt, Bucket, FileTree } from '@storagehub-sdk/msp-client';

import { TypeRegistry } from '@polkadot/types';

interface FileManagerProps {
  walletClient: WalletClient | null;
  publicClient: PublicClient | null;
  walletAddress: string | null;
  mspClient: MspClient | null;
  storageHubClient: StorageHubClient | null;
}

interface FileUploadState {
  file: File | null;
  /**
   * Fingerprint of the *selected* file before encryption.
   * If encryption is enabled, the uploaded fingerprint will be computed later from the encrypted bytes.
   */
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

interface FileDeleteState {
  deletingFiles: Set<string>;
  deleteError: string | null;
}

type UploadStep = 1 | 2 | 3;
type EncryptionMode = 'none' | 'password' | 'signature';

type DecryptUiState =
  | { open: false }
  | {
    open: true;
    fileName: string;
    encryptedBytes: Uint8Array;
    ikm: 'password' | 'signature';
    // Keep only what we need for UI decisions; decryptFile will re-parse internally.
    hasChallenge: boolean;
  };

export function FileManager({
  walletClient,
  publicClient,
  walletAddress,
  mspClient,
  storageHubClient,
}: FileManagerProps) {
  const bucketSelectId = useId();
  const folderNameInputId = useId();
  const encPasswordInputId = useId();
  const decPasswordInputId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [uploadStep, setUploadStep] = useState<UploadStep>(1);
  const [encryptionMode, setEncryptionMode] = useState<EncryptionMode>('none');
  const [encryptionPassword, setEncryptionPassword] = useState('');
  const [uploadStageLabel, setUploadStageLabel] = useState<string | null>(null);

  // Signature message params: keep stable to ensure recoverability across environments.
  const ENC_APP_NAME = 'StorageHub';
  const ENC_DOMAIN = 'storagehub-sdk-demo';
  const ENC_VERSION = 1;
  const ENC_PURPOSE = 'Encrypt file for StorageHub upload';

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

  const [deleteState, setDeleteState] = useState<FileDeleteState>({
    deletingFiles: new Set(),
    deleteError: null,
  });

  const [decryptState, setDecryptState] = useState<DecryptUiState>({ open: false });
  const [decryptPassword, setDecryptPassword] = useState('');
  const [isDecrypting, setIsDecrypting] = useState(false);

  const downloadBytes = (bytes: Uint8Array, filename: string, contentType?: string) => {
    const blob = new Blob([bytes], { type: contentType || 'application/octet-stream' });
    const downloadUrl = URL.createObjectURL(blob);
    const downloadLink = document.createElement('a');
    downloadLink.href = downloadUrl;
    downloadLink.download = filename;
    document.body.appendChild(downloadLink);
    downloadLink.click();
    document.body.removeChild(downloadLink);
    URL.revokeObjectURL(downloadUrl);
  };

  const inferredType = (node: FileTree): 'file' | 'folder' => {
    const n = node as unknown as Record<string, unknown>;
    if (n.type === 'file' || typeof (n as { fileKey?: unknown }).fileKey === 'string') return 'file';
    if (n.type === 'folder' || Array.isArray((n as { children?: unknown }).children)) return 'folder';
    return 'file'; // default: treat unknown as file to keep actions available
  };

  const normalizeTreeChildren = (resp: unknown): FileTree[] => {
    const r = resp as Record<string, unknown> | null;
    if (!r) return [];

    // Observed shapes:
    // - { files: [ { children: [...] } ] }
    // - { files: [ { tree: { children: [...] } } ] }
    // - { tree: { children: [...] } }
    const files = (r as { files?: unknown }).files;
    const tree = (r as { tree?: unknown }).tree as Record<string, unknown> | undefined;

    const first = Array.isArray(files) && files.length > 0 ? (files[0] as Record<string, unknown>) : undefined;
    const firstTree = first && typeof first === 'object' ? ((first as { tree?: unknown }).tree as Record<string, unknown> | undefined) : undefined;

    const root = (tree ?? firstTree ?? first) as Record<string, unknown> | undefined;
    const children = root?.children;
    if (Array.isArray(children)) return children as FileTree[];

    // Fallback: if response already contains a flat list, return it.
    if (Array.isArray(files)) return files as FileTree[];
    return [];
  };

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

    // Reset wizard steps on new selection.
    setUploadStep(1);

    // Compute fingerprint
    setUploadState(prev => ({ ...prev, isComputing: true }));

    try {
      await initWasm();

      const fileManager = new StorageHubFileManager({
        size: file.size,
        stream: () => {
          const body = new Response(file).body;
          if (!body) throw new Error('File stream is not available');
          return body as ReadableStream<Uint8Array>;
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

      // Get MSP information
      const info = await mspClient.info.getInfo();
      const mspId = (info.mspId || '') as `0x${string}`;

      // Get available value propositions from the MSP
      const valuePropositions = await mspClient.info.getValuePropositions();

      // Use the first available value proposition
      const valuePropId = (valuePropositions[0]?.id || '') as `0x${string}`;

      // Validate we have all required identifiers
      if (!mspId || !valuePropId) {
        console.error('MSP ID:', mspId, 'Value Prop ID:', valuePropId);
        throw new Error('Missing MSP identifiers');
      }

      const bucketId = await storageHubClient.deriveBucketId(walletAddress as `0x${string}`, bucketState.bucketName);

      const txHash = await storageHubClient.createBucket(
        mspId,
        bucketState.bucketName,
        false,
        valuePropId,
        undefined
      );

      console.log('Bucket creation transaction submitted:', txHash);

      if (!txHash) {
        throw new Error('createBucket did not return a transaction hash');
      }

      if (!publicClient) {
        throw new Error('Public client not available');
      }

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });

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
  const loadFiles = async (bucketId: string, path = '') => {
    if (!mspClient) {
      console.warn('⚠️ MSP client not available, cannot load files');
      return;
    }

    setFileBrowserState(prev => ({ ...prev, isLoading: true, error: null }));

    try {

      const fileListResponse = await mspClient.buckets.getFiles(bucketId, path ? { path } : undefined);
      console.log('📦 fileListResponse:', fileListResponse);

      const extractedFiles = normalizeTreeChildren(fileListResponse);

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
    if (!uploadState.file || !mspClient || !storageHubClient || !walletAddress || !selectedBucketId) return;

    setUploadState(prev => ({ ...prev, isUploading: true, error: null }));
    setUploadStageLabel(null);

    try {
      await initWasm();

      // Use the selected upload path or default to root
      const basePath = uploadLocationState.selectedPath || '';
      const outputFileName =
        encryptionMode === 'none' ? uploadState.file.name : `${uploadState.file.name}.enc`;
      const fileLocation = basePath ? `${basePath}/${outputFileName}` : outputFileName;

      setUploadState(prev => ({ ...prev, uploadProgress: 5 }));

      // Build the Blob that we will actually upload.
      let uploadBlob: Blob = uploadState.file;
      if (encryptionMode !== 'none') {
        if (encryptionMode === 'password') {
          if (!encryptionPassword) {
            throw new Error('Password is required to encrypt the file');
          }
          setUploadStageLabel('Generating encryption keys (password)…');
        } else {
          if (!walletClient) throw new Error('Wallet client not available for signature encryption');
          setUploadStageLabel('Preparing signature message…');
        }

        const chainId =
          walletClient && 'getChainId' in walletClient ? await walletClient.getChainId() : 0;

        const keysPromise =
          encryptionMode === 'password'
            ? await generateEncryptionKey({ kind: 'password', password: encryptionPassword })
            : (() => {
                const { message, challenge } = IKM.createEncryptionKeyMessage(
                  ENC_APP_NAME,
                  ENC_DOMAIN,
                  ENC_VERSION,
                  ENC_PURPOSE,
                  chainId,
                  walletAddress as `0x${string}`
                );
                return generateEncryptionKey({
                  kind: 'signature',
                  walletClient: walletClient as WalletClient,
                  account: walletAddress as `0x${string}`,
                  message,
                  challenge
                });
              })();
        const keys = await keysPromise;

        setUploadState(prev => ({ ...prev, uploadProgress: 10 }));
        setUploadStageLabel('Encrypting file…');

        const ts = new TransformStream<Uint8Array, Uint8Array>();
        const encryptedBlobP = new Response(ts.readable).blob();

        const inputBody = new Response(uploadState.file).body;
        if (!inputBody) throw new Error('File stream is not available for encryption');

        await encryptFile({
          input: inputBody as ReadableStream<Uint8Array>,
          output: ts.writable,
          dek: keys.dek,
          baseNonce: keys.baseNonce,
          header: keys.header,
          onProgress: ({ bytesProcessed }) => {
            const total = uploadState.file?.size ?? 1;
            const pct = Math.min(99, Math.floor((bytesProcessed / total) * 100));
            // Map encryption into 10..25% of the overall bar.
            const mapped = 10 + Math.floor((pct / 100) * 15);
            setUploadState(prev => ({ ...prev, uploadProgress: Math.max(prev.uploadProgress, mapped) }));
          }
        });

        uploadBlob = await encryptedBlobP;
        setUploadState(prev => ({ ...prev, uploadProgress: 25 }));
      }

      if (!uploadBlob.size || uploadBlob.size <= 0) {
        throw new Error(`Invalid upload size: ${uploadBlob.size}`);
      }

      // Create FileManager for the *actual upload bytes* (plaintext or encrypted).
      const fileManager = new StorageHubFileManager({
        size: uploadBlob.size,
        stream: () => {
          const body = new Response(uploadBlob).body;
          if (!body) throw new Error('Upload blob stream is not available');
          return body as ReadableStream<Uint8Array>;
        }
      });

      // Get file info from FileManager (like sdk-precompiles)
      setUploadStageLabel('Computing upload fingerprint…');
      const fingerprint = await fileManager.getFingerprint();
      const fileSizeNumber = fileManager.getFileSize();

      if (fileSizeNumber === undefined || fileSizeNumber === null) {
        throw new Error(`FileManager.getFileSize() returned ${fileSizeNumber}`);
      }

      const fileSize = BigInt(fileSizeNumber);

      // Create TypeRegistry and types for file key computation (like sdk-precompiles)
      const registry = new TypeRegistry();
      // Derive parameter types from the core FileManager method to avoid version mismatches
      type FileManagerOwner = Parameters<StorageHubFileManager["computeFileKey"]>[0];
      type FileManagerBucket = Parameters<StorageHubFileManager["computeFileKey"]>[1];
      const owner = registry.createType("AccountId20", walletAddress) as unknown as FileManagerOwner;


      // Ensure bucket ID is properly formatted as 32-byte hex string
      let bucketIdForH256 = selectedBucketId;
      if (!bucketIdForH256.startsWith('0x')) {
        bucketIdForH256 = `0x${bucketIdForH256}`;
      }
      // H256 expects exactly 64 hex chars (32 bytes) after 0x
      if (bucketIdForH256.length !== 66) { // 0x + 64 hex chars = 66 total
        console.error('❌ Invalid bucket ID length for H256:', bucketIdForH256.length, 'expected 66');
        throw new Error(`Invalid bucket ID format: ${bucketIdForH256} (length: ${bucketIdForH256.length})`);
      }

      const bucketIdH256 = registry.createType("H256", bucketIdForH256) as unknown as FileManagerBucket;
      // File key is computed by the MSP backend during upload
      await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      setUploadState(prev => ({ ...prev, uploadProgress: 25 }));

      // Issue storage request
      // Derive IDs from MSP client
      const info = await mspClient.info.getInfo();
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
      // Ensure bucket ID has 0x prefix for storage request
      const bucketIdForStorageRequest = selectedBucketId.startsWith('0x') ? selectedBucketId : `0x${selectedBucketId}`;

      let storageRequestTxHash: `0x${string}` | undefined;
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

      if (!storageRequestTxHash) {
        throw new Error('No transaction hash returned from issueStorageRequest');
      }

      if (!publicClient) {
        throw new Error('Public client not available');
      }

      const storageRequestReceipt = await publicClient.waitForTransactionReceipt({
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

      let uploadReceipt: UploadReceipt | undefined;
      try {
        // Upload file to MSP (use exact same pattern as sdk-precompiles line 245-251)
        const fileBlob = uploadBlob; // upload the exact bytes we fingerprinted
        const fileKeyHex = finalFileKey.toHex();

        await new Promise(resolve => setTimeout(resolve, 3000)); // Add a 3 second delay before uploading

        if (!walletAddress) {
          throw new Error('Wallet address not available');
        }

        uploadReceipt = await mspClient.files.uploadFile(
          selectedBucketId,
          fileKeyHex,
          fileBlob,
          walletAddress,
          fileLocation
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

      const children = normalizeTreeChildren(fileListResponse);
      const subfolders = children.filter((child) => inferredType(child) === 'folder');
      setUploadLocationState(prev => ({
        ...prev,
        availableFolders: subfolders,
        navigationHistory: currentPath ? [currentPath] : ['/'],
        isLoadingFolders: false
      }));
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

      const children = normalizeTreeChildren(fileListResponse);
      const subfolders = children.filter((child) => inferredType(child) === 'folder');
      setUploadLocationState(prev => ({
        ...prev,
        selectedPath: newPath,
        navigationHistory: [...prev.navigationHistory, newPath],
        availableFolders: subfolders,
        isLoadingFolders: false
      }));
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

        const children = normalizeTreeChildren(fileListResponse);
        const subfolders = children.filter((child) => inferredType(child) === 'folder');
        setUploadLocationState(prev => ({
          ...prev,
          selectedPath: newPath,
          navigationHistory: newHistory,
          availableFolders: subfolders
        }));
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

      const children = normalizeTreeChildren(fileListResponse);
      const subfolders = children.filter((child) => inferredType(child) === 'folder');
      setUploadLocationState(prev => ({
        ...prev,
        selectedPath: '',
        navigationHistory: ['/'],
        availableFolders: subfolders
      }));
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
    if (!mspClient || !('fileKey' in file) || !file.fileKey) {
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
      // If file looks like an encrypted blob (StorageHub File header), offer decrypt.
      try {
        const { header } = readEncryptionHeader(combinedArray);
        const ikm = header.ikm;
        const hasChallenge = !!header.challenge;
        if (ikm === 'password' || ikm === 'signature') {
          setDecryptPassword('');
          setDecryptState({
            open: true,
            fileName: file.name,
            encryptedBytes: combinedArray,
            ikm,
            hasChallenge
          });
          return;
        }
      } catch {
        // Not encrypted in our format; fall through.
      }

      downloadBytes(combinedArray, file.name, downloadResult.contentType ?? undefined);
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

  // File delete function
  const deleteFile = async (file: FileTree) => {
    if (!mspClient || !storageHubClient || !publicClient) {
      console.error('Cannot delete: missing MSP client or StorageHub client');
      return;
    }

    if (!(file.type === 'file' && 'fileKey' in file && file.fileKey)) {
      console.error('Cannot delete: not a file or missing file key');
      return;
    }

    const fileKey = file.fileKey;
    const bucketId = fileBrowserState.selectedBucketId;
    if (!bucketId) {
      console.error('Cannot delete: no bucket selected');
      return;
    }

    setDeleteState(prev => ({
      ...prev,
      deletingFiles: new Set([...prev.deletingFiles, fileKey]),
      deleteError: null,
    }));

    const to0x = (hex: string): `0x${string}` => (hex.startsWith('0x') ? hex : (`0x${hex}`)) as `0x${string}`;

    try {
      // Retrieve file info from MSP to build the on-chain delete request
      const info = await mspClient.files.getFileInfo(bucketId, fileKey);

      const coreInfo: CoreFileInfo = {
        fileKey: to0x(info.fileKey),
        fingerprint: to0x(info.fingerprint),
        bucketId: to0x(info.bucketId),
        location: info.location,
        size: BigInt(info.size),
        blockHash: to0x(info.blockHash),
      };

      const txHash = await storageHubClient.requestDeleteFile(coreInfo);

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      if (receipt.status !== 'success') {
        throw new Error('Deletion transaction failed');
      }

      // Refresh files after successful tx (status may show deletionInProgress until finalized)
      if (fileBrowserState.selectedBucketId) {
        await loadFiles(fileBrowserState.selectedBucketId, fileBrowserState.currentPath);
      }
    } catch (error: unknown) {
      console.error('❌ Delete failed:', error);
      setDeleteState(prev => ({
        ...prev,
        deleteError: error instanceof Error ? error.message : 'Delete failed',
      }));
    } finally {
      setDeleteState(prev => ({
        ...prev,
        deletingFiles: new Set([...prev.deletingFiles].filter(key => key !== fileKey)),
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
    setUploadStep(1);
    setEncryptionMode('none');
    setEncryptionPassword('');
    setUploadStageLabel(null);
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
            type="button"
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

        {/* Mini wizard stepper */}
        <div className="flex items-center gap-2 text-xs text-gray-400">
          <span className={uploadStep === 1 ? 'text-blue-400 font-medium' : ''}>1) Select</span>
          <span>→</span>
          <span className={uploadStep === 2 ? 'text-blue-400 font-medium' : ''}>2) Encryption</span>
          <span>→</span>
          <span className={uploadStep === 3 ? 'text-blue-400 font-medium' : ''}>3) Review</span>
        </div>

        {/* Bucket Selection */}
        <div>
          <label htmlFor={bucketSelectId} className="block text-sm font-medium text-gray-300 mb-2">
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
              id={bucketSelectId}
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
              type="button"
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
                void handleFileSelect(files[0]);
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
                  type="button"
                  onClick={clearUpload}
                  className="text-gray-400 hover:text-red-400"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>

              {uploadStep === 1 && (
                <>
                  {uploadState.isComputing && (
                    <div className="flex items-center gap-2 text-blue-400">
                      <Hash className="h-4 w-4 animate-spin" />
                      <span className="text-sm">Computing original fingerprint…</span>
                    </div>
                  )}

                  {uploadState.fingerprint && (
                    <div className="space-y-2">
                      <div className="flex items-center gap-2 text-green-400">
                        <CheckCircle className="h-4 w-4" />
                        <span className="text-sm">Original fingerprint computed</span>
                      </div>
                      <div className="text-xs text-gray-400 font-mono break-all">
                        {uploadState.fingerprint}
                      </div>
                      <div className="text-xs text-gray-500">
                        If you enable encryption, the uploaded fingerprint will be computed from the encrypted bytes.
                      </div>
                    </div>
                  )}
                </>
              )}

              {/* Upload Location Selector */}
              {uploadStep === 1 && (
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
                        type="button"
                        onClick={openFolderBrowser}
                        className="flex items-center gap-1 px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                      >
                        <Folder className="h-3 w-3" />
                        Browse Folders
                      </button>

                      <button
                        type="button"
                        onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: true }))}
                        className="flex items-center gap-1 px-3 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
                      >
                        <Plus className="h-3 w-3" />
                        Create Folder
                      </button>

                      <button
                        type="button"
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

              {/* Wizard navigation + upload action */}
              <div className="flex gap-2">
                {uploadStep > 1 && (
                  <button
                    type="button"
                    onClick={() => setUploadStep((s) => (s === 2 ? 1 : 2))}
                    disabled={uploadState.isUploading}
                    className="flex-1 rounded-md bg-gray-700 px-4 py-2 text-sm font-medium text-gray-200 hover:bg-gray-600 disabled:bg-gray-900 disabled:cursor-not-allowed"
                  >
                    Back
                  </button>
                )}

                {uploadStep < 3 && (
                  <button
                    type="button"
                    onClick={() => setUploadStep((s) => (s === 1 ? 2 : 3))}
                    disabled={
                      uploadState.isUploading ||
                      uploadState.isComputing ||
                      !selectedBucketId ||
                      !uploadState.file ||
                      (uploadStep === 2 && encryptionMode === 'password' && !encryptionPassword) ||
                      (uploadStep === 2 && encryptionMode === 'signature' && !walletClient)
                    }
                    className="flex-1 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
                  >
                    Next
                  </button>
                )}

                {uploadStep === 3 && (
                  <button
                    type="button"
                    onClick={uploadFile}
                    disabled={uploadState.isUploading || !selectedBucketId}
                    className="flex-1 flex items-center justify-center gap-2 rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
                  >
                    <Upload className="h-4 w-4" />
                    {uploadState.isUploading ? `Uploading… ${uploadState.uploadProgress}%` : 'Upload'}
                  </button>
                )}
              </div>

              {uploadStep === 2 && (
                <div className="rounded-md border border-gray-700 bg-gray-900 p-3 space-y-3">
                  <div className="text-sm font-medium text-gray-200">Encryption</div>

                  <div className="space-y-2 text-sm">
                    <label className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="enc-mode"
                        checked={encryptionMode === 'none'}
                        onChange={() => setEncryptionMode('none')}
                      />
                      <span>No encryption</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="enc-mode"
                        checked={encryptionMode === 'password'}
                        onChange={() => setEncryptionMode('password')}
                      />
                      <span>Encrypt with password</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="enc-mode"
                        checked={encryptionMode === 'signature'}
                        onChange={() => setEncryptionMode('signature')}
                      />
                      <span>Encrypt with wallet signature</span>
                    </label>
                  </div>

                  {encryptionMode === 'password' && (
                    <div className="space-y-2">
                      <label htmlFor={encPasswordInputId} className="block text-xs text-gray-400">
                        Password
                      </label>
                      <input
                        id={encPasswordInputId}
                        type="password"
                        value={encryptionPassword}
                        onChange={(e) => setEncryptionPassword(e.target.value)}
                        className="w-full rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 focus:outline-none"
                        placeholder="Enter password to encrypt"
                      />
                      <div className="text-xs text-yellow-400">
                        If you lose this password, you won’t be able to decrypt the file.
                      </div>
                    </div>
                  )}

                  {encryptionMode === 'signature' && (
                    <div className="space-y-2 text-xs text-gray-300">
                      <div className="text-yellow-400">
                        MetaMask will prompt you to sign a message. This signature is used to derive encryption keys.
                      </div>
                      {!walletClient && (
                        <div className="text-red-400">Wallet client not available.</div>
                      )}
                      <div className="text-gray-400">
                        The SDK generates a file-specific random challenge (stored inside the encrypted file header) and includes it in the message.
                      </div>
                    </div>
                  )}
                </div>
              )}

              {uploadStep === 3 && (
                <div className="rounded-md border border-gray-700 bg-gray-900 p-3 space-y-2">
                  <div className="text-sm font-medium text-gray-200">Review</div>
                  <div className="text-xs text-gray-400">
                    <div>
                      <span className="text-gray-500">Upload name:</span>{' '}
                      <span className="font-mono">
                        {encryptionMode === 'none' ? uploadState.file.name : `${uploadState.file.name}.enc`}
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-500">Location:</span>{' '}
                      <span className="font-mono">{uploadLocationState.selectedPath || '/'}</span>
                    </div>
                    <div>
                      <span className="text-gray-500">Encryption:</span>{' '}
                      <span className="font-mono">{encryptionMode}</span>
                    </div>
                    {uploadStageLabel && (
                      <div className="mt-2 text-blue-400">
                        {uploadStageLabel}
                      </div>
                    )}
                  </div>
                </div>
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
                  void loadFiles(bucketId);
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
                  type="button"
                  onClick={() => {
                    if (fileBrowserState.selectedBucketId) {
                      void loadFiles(fileBrowserState.selectedBucketId, fileBrowserState.currentPath);
                    }
                  }}
                  disabled={fileBrowserState.isLoading}
                  className="px-4 py-2 text-sm bg-gray-700 text-gray-300 rounded-md hover:bg-gray-600 disabled:bg-gray-800 disabled:cursor-not-allowed transition-colors"
                >
                  {fileBrowserState.isLoading ? 'Loading...' : 'Refresh'}
                </button>

                {/* Back Button - only show if we're not at root */}
                {fileBrowserState.currentPath && fileBrowserState.currentPath !== '/' && (
                  <button
                    type="button"
                    onClick={() => {
                      // Navigate back one level
                      const pathParts = fileBrowserState.currentPath.split('/');
                      const parentPath = pathParts.slice(0, -1).join('/') || '';
                      if (fileBrowserState.selectedBucketId) {
                        void loadFiles(fileBrowserState.selectedBucketId, parentPath);
                      }
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
                  <div className="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-2" />
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
                    // biome-ignore lint/a11y/useSemanticElements: Cannot use button here as it contains nested buttons for actions
                    <div
                      key={`${file.name}-${index}`}
                      role="button"
                      tabIndex={0}
                      className={`p-4 hover:bg-gray-800 cursor-pointer transition-colors ${fileBrowserState.selectedFile === file ? 'bg-blue-900/20 border-l-4 border-blue-500' : ''
                        }`}
                      onClick={() => setFileBrowserState(prev => ({
                        ...prev,
                        selectedFile: prev.selectedFile === file ? null : file
                      }))}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          setFileBrowserState(prev => ({
                            ...prev,
                            selectedFile: prev.selectedFile === file ? null : file
                          }));
                        }
                      }}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          {inferredType(file) === 'folder' ? (
                            <Folder className="h-5 w-5 text-blue-400" />
                          ) : (
                            <File className="h-5 w-5 text-gray-400" />
                          )}
                          <div>
                            <div className="text-sm font-medium text-gray-200">{file.name}</div>
                            <div className="text-xs text-gray-500">
                              {inferredType(file) === 'file' ? (
                                <>
                                  {'sizeBytes' in file && typeof file.sizeBytes === 'number' ? `${(file.sizeBytes / 1024).toFixed(1)} KB` : 'Unknown size'}
                                  {'fileKey' in file && typeof file.fileKey === 'string' && (
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
                          {inferredType(file) === 'file' && 'fileKey' in file && file.fileKey && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                void downloadFile(file);
                              }}
                              disabled={downloadState.downloadingFiles.has(('fileKey' in file && file.fileKey) ? file.fileKey : '')}
                              className="px-3 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors"
                            >
                              {downloadState.downloadingFiles.has(('fileKey' in file && file.fileKey) ? file.fileKey : '') ? (
                                <>
                                  <div className="animate-spin h-3 w-3 border border-white border-t-transparent rounded-full inline mr-1" />
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
                          {inferredType(file) === 'file' && 'fileKey' in file && file.fileKey && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                void deleteFile(file);
                              }}
                              disabled={deleteState.deletingFiles.has(('fileKey' in file && file.fileKey) ? file.fileKey : '')}
                              className="px-3 py-1 text-xs bg-red-600 text-white rounded hover:bg-red-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors"
                            >
                              {deleteState.deletingFiles.has(('fileKey' in file && file.fileKey) ? file.fileKey : '') ? (
                                <>
                                  <div className="animate-spin h-3 w-3 border border-white border-t-transparent rounded-full inline mr-1" />
                                  Deleting...
                                </>
                              ) : (
                                <>
                                  <Trash2 className="h-3 w-3 inline mr-1" />
                                  Delete
                                </>
                              )}
                            </button>
                          )}
                          {inferredType(file) === 'folder' && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                const newPath = fileBrowserState.currentPath
                                  ? `${fileBrowserState.currentPath}/${file.name}`
                                  : file.name;
                                if (fileBrowserState.selectedBucketId) {
                                  void loadFiles(fileBrowserState.selectedBucketId, newPath);
                                }
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
                  type="button"
                  onClick={() => setDownloadState(prev => ({ ...prev, downloadError: null }))}
                  className="ml-auto text-red-400 hover:text-red-300"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
              <p className="text-sm text-red-300 mt-2">{downloadState.downloadError}</p>
            </div>
          )}

          {/* Delete Error */}
          {deleteState.deleteError && (
            <div className="p-4 bg-red-900/20 border border-red-900/50 rounded-lg">
              <div className="flex items-center gap-2 text-red-400">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm font-medium">Delete Failed</span>
                <button
                  type="button"
                  onClick={() => setDeleteState(prev => ({ ...prev, deleteError: null }))}
                  className="ml-auto text-red-400 hover:text-red-300"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
              <p className="text-sm text-red-300 mt-2">{deleteState.deleteError}</p>
            </div>
          )}

          {/* Selected File Info */}
          {fileBrowserState.selectedFile && (
            <div className="p-4 bg-gray-800 rounded-lg border border-gray-700">
              <h4 className="text-sm font-medium text-gray-200 mb-2">File Information</h4>
              <div className="space-y-1 text-xs text-gray-400">
                <div><strong>Name:</strong> {fileBrowserState.selectedFile.name}</div>
                <div><strong>Type:</strong> {inferredType(fileBrowserState.selectedFile)}</div>
                {'sizeBytes' in fileBrowserState.selectedFile && typeof fileBrowserState.selectedFile.sizeBytes === 'number' && (
                  <div><strong>Size:</strong> {(fileBrowserState.selectedFile.sizeBytes / 1024).toFixed(2)} KB</div>
                )}
                {'fileKey' in fileBrowserState.selectedFile && typeof fileBrowserState.selectedFile.fileKey === 'string' && (
                  <div><strong>File Key:</strong> {fileBrowserState.selectedFile.fileKey}</div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Decrypt Download Modal */}
      {decryptState.open && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-900 border border-gray-700 rounded-lg p-6 w-full max-w-lg">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-200">Encrypted file detected</h3>
              <button
                type="button"
                onClick={() => setDecryptState({ open: false })}
                className="text-gray-400 hover:text-white"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="space-y-3 text-sm text-gray-300">
              <div>
                <span className="text-gray-400">File:</span> <span className="font-mono">{decryptState.fileName}</span>
              </div>
              <div>
                <span className="text-gray-400">Key type:</span> <span className="font-mono">{decryptState.ikm}</span>
              </div>
            </div>

            {decryptState.ikm === 'password' && (
              <div className="mt-4 space-y-2">
                <label className="block text-xs text-gray-400" htmlFor={decPasswordInputId}>
                  Password
                </label>
                <input
                  id={decPasswordInputId}
                  type="password"
                  value={decryptPassword}
                  onChange={(e) => setDecryptPassword(e.target.value)}
                  className="w-full rounded-md border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 focus:border-blue-500 focus:outline-none"
                  placeholder="Enter password to decrypt"
                />
              </div>
            )}

            {decryptState.ikm === 'signature' && (
              <div className="mt-4 text-xs text-gray-400">
                You’ll be prompted to sign a message to derive the decryption keys.
              </div>
            )}

            <div className="mt-6 flex gap-2">
              <button
                type="button"
                onClick={() => {
                  downloadBytes(decryptState.encryptedBytes, decryptState.fileName);
                  setDecryptState({ open: false });
                }}
                className="flex-1 px-4 py-2 text-sm bg-gray-700 text-gray-200 rounded hover:bg-gray-600"
              >
                Download encrypted
              </button>

              <button
                type="button"
                disabled={isDecrypting || (decryptState.ikm === 'password' && !decryptPassword)}
                onClick={async () => {
                  try {
                    setIsDecrypting(true);
                    const ts = new TransformStream<Uint8Array, Uint8Array>();
                    const decryptedP = new Response(ts.readable).arrayBuffer().then((b) => new Uint8Array(b));

                    const chainId = walletClient ? await walletClient.getChainId() : 0;
                    const inputBody = new Response(decryptState.encryptedBytes).body;
                    if (!inputBody) throw new Error('Encrypted bytes stream is not available for decryption');
                    await decryptFile({
                      input: inputBody as ReadableStream<Uint8Array>,
                      output: ts.writable,
                      getIkm: async (hdr) => {
                        if (hdr.ikm === 'password') {
                          return IKM.fromPassword(decryptPassword).unwrap();
                        }
                        if (hdr.ikm === 'signature') {
                          if (!walletClient || !walletAddress) throw new Error('Wallet not connected');
                          if (!hdr.challenge) throw new Error('Missing challenge in encrypted header');
                          const { message } = IKM.createEncryptionKeyMessage(
                            ENC_APP_NAME,
                            ENC_DOMAIN,
                            ENC_VERSION,
                            ENC_PURPOSE,
                            chainId,
                            walletAddress as `0x${string}`,
                            hdr.challenge as any
                          );
                          const signature = await walletClient.signMessage({
                            account: walletAddress as `0x${string}`,
                            message
                          });
                          return IKM.fromSignature(signature).unwrap();
                        }
                        throw new Error('Unknown IKM type');
                      }
                    });

                    const decrypted = await decryptedP;
                    const outName = decryptState.fileName.endsWith('.enc')
                      ? decryptState.fileName.slice(0, -4)
                      : `${decryptState.fileName}.decrypted`;
                    downloadBytes(decrypted, outName);
                    setDecryptState({ open: false });
                  } catch (e) {
                    console.error('Decrypt failed:', e);
                    setDownloadState(prev => ({
                      ...prev,
                      downloadError: e instanceof Error ? e.message : 'Decrypt failed'
                    }));
                  } finally {
                    setIsDecrypting(false);
                  }
                }}
                className="flex-1 px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
              >
                {isDecrypting ? 'Decrypting…' : 'Decrypt & download'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Folder Browser Modal */}
      {uploadLocationState.isNavigating && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-900 border border-gray-700 rounded-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-hidden">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-200">Select Upload Location</h3>
              <button
                type="button"
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
                type="button"
                onClick={resetToRoot}
                className="flex items-center gap-1 px-2 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                <Hash className="h-3 w-3" />
                Root
              </button>

              {uploadLocationState.navigationHistory.length > 1 && (
                <button
                  type="button"
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
                  <div className="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-2" />
                  <p>Loading folders...</p>
                </div>
              ) : uploadLocationState.availableFolders.length === 0 ? (
                <div className="text-center py-8 text-gray-500">
                  <Folder className="h-12 w-12 mx-auto mb-2 opacity-50" />
                  <p>No folders found</p>
                </div>
              ) : (
                uploadLocationState.availableFolders.map((folder, index) => (
                  <button
                    key={`${folder.name}-${index}`}
                    type="button"
                    onClick={() => navigateToFolder(folder.name)}
                    className="flex items-center gap-3 p-3 bg-gray-800 rounded hover:bg-gray-700 cursor-pointer transition-colors w-full text-left"
                  >
                    <Folder className="h-5 w-5 text-blue-400" />
                    <span className="text-sm text-gray-200">{folder.name}</span>
                    <span className="text-xs text-gray-500 ml-auto">→</span>
                    <span className="text-xs text-gray-600 ml-2">
                      (in {uploadLocationState.selectedPath || 'root'})
                    </span>
                  </button>
                ))
              )}
            </div>

            {/* Action Buttons */}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: true }))}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-700"
              >
                <Plus className="h-4 w-4" />
                Create New Folder
              </button>

              <button
                type="button"
                onClick={selectCurrentPath}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                <CheckCircle className="h-4 w-4" />
                Select This Location
              </button>

              <button
                type="button"
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
                type="button"
                onClick={() => setUploadLocationState(prev => ({ ...prev, showFolderCreator: false, newFolderName: '' }))}
                className="text-gray-400 hover:text-white"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label htmlFor={folderNameInputId} className="block text-sm font-medium text-gray-300 mb-2">
                  Folder Name
                </label>
                <input
                  id={folderNameInputId}
                  type="text"
                  value={uploadLocationState.newFolderName}
                  onChange={(e) => setUploadLocationState(prev => ({ ...prev, newFolderName: e.target.value }))}
                  placeholder="Enter folder name"
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-gray-100 focus:border-blue-500 focus:outline-none"
                />
              </div>

              <div className="text-sm text-gray-400">
                <span>Will be created in: </span>
                <span className="font-mono">{uploadLocationState.selectedPath || '/'}</span>
              </div>

              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={createNewFolder}
                  disabled={!uploadLocationState.newFolderName.trim()}
                  className="flex-1 px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed"
                >
                  Create Folder
                </button>

                <button
                  type="button"
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