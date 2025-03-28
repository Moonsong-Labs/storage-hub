// // Tests for the Storage Hub file system pallet
// use crate::{create_client, get_keypair, TestConfig, DEFAULT_TIMEOUT};
// use anyhow::Result;
// use subxt::utils::AccountId32;
// use subxt_signer::sr25519::dev;
// use tracing::info;
//
// // Define the storage_hub module directly here to avoid import issues
// #[subxt::subxt(runtime_metadata_path = "./storage-hub.scale")]
// pub mod storage_hub {}
//
// /// Test file creation in the Storage Hub
// #[tokio::test]
// async fn test_file_creation() -> Result<()> {
//     // Initialize tracing
//     tracing_subscriber::fmt::init();
//
//     let config = TestConfig::default();
//     let client = create_client(&config).await?;
//
//     // Alice will create a file
//     let signer = dev::alice();
//     let alice_account: AccountId32 = dev::alice().public_key().into();
//
//     // File metadata
//     let file_name = "test_file.txt";
//     let file_size = 1024; // 1KB
//     let file_type = "text/plain";
//
//     // Prepare file creation transaction
//     // Note: Adjust this to match the actual file_system pallet API
//     let tx = storage_hub::tx().file_system().create_file(
//         file_name.as_bytes().to_vec(),
//         file_size,
//         file_type.as_bytes().to_vec(),
//         None, // No specific provider
//         None, // Default encryption
//     );
//
//     // Sign and submit the creation, then wait for finalization
//     let events = client
//         .tx()
//         .sign_and_submit_then_watch_default(&tx, &signer)
//         .await?
//         .wait_for_finalized_success()
//         .await?;
//
//     info!(
//         "File creation transaction finalized in block: {}",
//         events.block_hash()
//     );
//
//     // Check for the file creation event
//     // Note: Adjust to match the actual event structure
//     let file_created_event =
//         events.find_first::<storage_hub::file_system::events::FileCreated>()?;
//
//     if let Some(event) = file_created_event {
//         info!("File created successfully with ID: {:?}", event.file_id);
//
//         // Additional validation could be performed here
//         // such as checking file ownership, size, etc.
//     } else {
//         anyhow::bail!("File creation event not found");
//     }
//
//     Ok(())
// }
//
// /// Test uploading file chunks
// #[tokio::test]
// async fn test_upload_chunks() -> Result<()> {
//     // Initialize tracing
//     tracing_subscriber::fmt::init();
//
//     let config = TestConfig::default();
//     let client = create_client(&config).await?;
//
//     // Alice will upload chunks
//     let signer = dev::alice();
//
//     // First, create a file (simplified from previous test)
//     let file_name = "chunk_test.txt";
//     let file_size = 4096; // 4KB
//     let file_type = "text/plain";
//
//     // Create the file
//     let create_tx = storage_hub::tx().file_system().create_file(
//         file_name.as_bytes().to_vec(),
//         file_size,
//         file_type.as_bytes().to_vec(),
//         None,
//         None,
//     );
//
//     let create_events = client
//         .tx()
//         .sign_and_submit_then_watch_default(&create_tx, &signer)
//         .await?
//         .wait_for_finalized_success()
//         .await?;
//
//     // Get the file ID from the creation event
//     let file_created_event = create_events
//         .find_first::<storage_hub::file_system::events::FileCreated>()?
//         .ok_or_else(|| anyhow::anyhow!("File creation event not found"))?;
//
//     let file_id = file_created_event.file_id;
//     info!("Created file with ID: {:?}", file_id);
//
//     // Now upload some chunks (adjust to match actual API)
//     let chunk_size = 1024; // 1KB per chunk
//     let chunk_data = vec![0u8; chunk_size]; // Sample data filled with zeros
//
//     for chunk_index in 0..4 {
//         // Upload each chunk (adjust to match actual API)
//         let upload_tx = storage_hub::tx().file_system().upload_chunk(
//             file_id.clone(),
//             chunk_index,
//             chunk_data.clone(),
//         );
//
//         let upload_events = client
//             .tx()
//             .sign_and_submit_then_watch_default(&upload_tx, &signer)
//             .await?
//             .wait_for_finalized_success()
//             .await?;
//
//         // Check for chunk upload event
//         let chunk_uploaded_event =
//             upload_events.find_first::<storage_hub::file_system::events::ChunkUploaded>()?;
//
//         if let Some(event) = chunk_uploaded_event {
//             info!(
//                 "Uploaded chunk {} for file ID {:?}",
//                 event.chunk_index, event.file_id
//             );
//         } else {
//             anyhow::bail!("Chunk upload event not found for chunk {}", chunk_index);
//         }
//     }
//
//     // Now finalize the file upload (adjust to match actual API)
//     let finalize_tx = storage_hub::tx()
//         .file_system()
//         .finalize_file(file_id.clone());
//
//     let finalize_events = client
//         .tx()
//         .sign_and_submit_then_watch_default(&finalize_tx, &signer)
//         .await?
//         .wait_for_finalized_success()
//         .await?;
//
//     // Check for file finalized event
//     let file_finalized_event =
//         finalize_events.find_first::<storage_hub::file_system::events::FileFinalized>()?;
//
//     if let Some(event) = file_finalized_event {
//         info!("File finalized with ID: {:?}", event.file_id);
//     } else {
//         anyhow::bail!("File finalization event not found");
//     }
//
//     Ok(())
// }
