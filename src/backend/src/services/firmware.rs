//! Firmware update service
//!
//! Handles firmware file management and operations independent of HTTP concerns.

use crate::{config::AppConfig, omnect_device_service_client::DeviceServiceClient};
use actix_multipart::form::tempfile::TempFile;
use anyhow::{Context, Result};
use log::{debug, error};
use std::{fs, os::unix::fs::PermissionsExt};

#[cfg(any(test, feature = "mock"))]
use std::sync::{LazyLock, Mutex, MutexGuard};

#[cfg(any(test, feature = "mock"))]
#[allow(dead_code)]
static DATA_FOLDER_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Service for firmware update file operations
pub struct FirmwareService;

impl FirmwareService {
    /// Acquire a lock for data folder operations (test-only)
    ///
    /// This ensures that tests modifying the data folder don't interfere with each other
    #[cfg(any(test, feature = "mock"))]
    #[allow(dead_code)]
    pub fn lock_for_test() -> MutexGuard<'static, ()> {
        DATA_FOLDER_LOCK.lock().unwrap()
    }

    /// Handle uploaded firmware file - clears data folder and persists the file
    ///
    /// # Arguments
    /// * `tmp_file` - The temporary uploaded file
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn handle_uploaded_firmware(tmp_file: TempFile) -> Result<()> {
        // Clear data folder (non-critical if it fails)
        if let Err(e) = Self::clear_data_folder() {
            error!("failed to clear data folder: {e:#}");
            // Continue anyway as this is not critical
        }

        let local_update_file = &AppConfig::get().paths.local_update_file;
        let tmp_update_file = &AppConfig::get().paths.tmp_update_file;

        // 1. store tempfile in temp dir (cannot be persisted across filesystems)
        tmp_file
            .file
            .persist(tmp_update_file)
            .context("failed to persist temporary file")?;

        // 2. copy to local container filesystem
        fs::copy(tmp_update_file, local_update_file)
            .context("failed to copy firmware to data dir")?;

        // 3. allow host to access the file
        let metadata =
            fs::metadata(local_update_file).context("failed to get firmware metadata")?;
        let mut perm = metadata.permissions();
        perm.set_mode(0o750);
        fs::set_permissions(local_update_file, perm)
            .context("failed to set firmware permissions")?;

        Ok(())
    }

    /// Load the firmware update file via the device service client
    ///
    /// # Arguments
    /// * `service_client` - Device service client for loading the update
    ///
    /// # Returns
    /// Result with the response data from the device service
    pub async fn load_update<SC: DeviceServiceClient>(service_client: &SC) -> Result<String> {
        use crate::omnect_device_service_client::LoadUpdate;

        service_client
            .load_update(LoadUpdate {
                update_file_path: AppConfig::get().paths.host_update_file.clone(),
            })
            .await
    }

    /// Run the firmware update via the device service client
    ///
    /// # Arguments
    /// * `service_client` - Device service client for running the update
    /// * `run_update` - The update configuration
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn run_update<ServiceClient: DeviceServiceClient>(
        service_client: &ServiceClient,
        run_update: crate::omnect_device_service_client::RunUpdate,
    ) -> Result<()> {
        service_client.run_update(run_update).await
    }

    /// Clear all files in the data folder
    fn clear_data_folder() -> Result<()> {
        debug!("clear_data_folder() called");
        let data_dir = fs::read_dir(&AppConfig::get().paths.data_dir)?;
        for entry in data_dir {
            let entry = entry?;
            if entry.path().is_file() {
                fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write as _;

    #[cfg(feature = "mock")]
    use mockall_double::double;

    #[cfg(feature = "mock")]
    #[double]
    use crate::omnect_device_service_client::DeviceServiceClient;

    mod clear_data_folder {
        use super::*;

        #[test]
        fn removes_all_files() {
            let _lock = FirmwareService::lock_for_test();
            let data_path = &AppConfig::get().paths.data_dir;

            // Ensure directory exists
            fs::create_dir_all(data_path).expect("should create data dir");

            // Create some test files
            File::create(data_path.join("file1.txt"))
                .expect("should create file1")
                .write_all(b"test")
                .expect("should write");
            File::create(data_path.join("file2.txt"))
                .expect("should create file2")
                .write_all(b"test")
                .expect("should write");

            // Verify files exist
            assert!(data_path.join("file1.txt").exists());
            assert!(data_path.join("file2.txt").exists());

            // Clear folder
            FirmwareService::clear_data_folder().expect("should clear folder");

            // Verify files are deleted
            assert!(!data_path.join("file1.txt").exists());
            assert!(!data_path.join("file2.txt").exists());
        }

        #[test]
        fn succeeds_with_empty_directory() {
            let _lock = FirmwareService::lock_for_test();
            let data_path = &AppConfig::get().paths.data_dir;

            // Ensure directory exists and is empty
            fs::create_dir_all(data_path).expect("should create data dir");

            // Clear folder when already empty
            let result = FirmwareService::clear_data_folder();
            assert!(result.is_ok());
        }

        #[test]
        fn preserves_subdirectories() {
            let _lock = FirmwareService::lock_for_test();
            let data_path = &AppConfig::get().paths.data_dir;

            // Ensure directory exists
            fs::create_dir_all(data_path).expect("should create data dir");

            // Create a subdirectory
            let subdir = data_path.join("subdir");
            fs::create_dir_all(&subdir).expect("should create subdir");

            // Create a file in root
            File::create(data_path.join("file.txt"))
                .expect("should create file")
                .write_all(b"test")
                .expect("should write");

            // Clear folder
            FirmwareService::clear_data_folder().expect("should clear folder");

            // File should be deleted
            assert!(!data_path.join("file.txt").exists());

            // Subdirectory should still exist (only files are removed)
            assert!(subdir.exists());
            assert!(subdir.is_dir());

            // Cleanup
            let _ = fs::remove_dir(&subdir);
        }
    }

    mod load_update {
        use super::*;
        use crate::omnect_device_service_client::LoadUpdate;

        #[tokio::test]
        async fn forwards_request_to_device_service() {
            let mut device_mock = DeviceServiceClient::default();

            device_mock
                .expect_load_update()
                .withf(|req: &LoadUpdate| {
                    req.update_file_path == AppConfig::get().paths.host_update_file
                })
                .times(1)
                .returning(|_| Box::pin(async { Ok("update loaded successfully".to_string()) }));

            let result = FirmwareService::load_update(&device_mock).await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "update loaded successfully");
        }

        #[tokio::test]
        async fn returns_error_on_device_service_failure() {
            let mut device_mock = DeviceServiceClient::default();

            device_mock
                .expect_load_update()
                .returning(|_| Box::pin(async { Err(anyhow::anyhow!("device service error")) }));

            let result = FirmwareService::load_update(&device_mock).await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("device service error")
            );
        }
    }

    mod run_update {
        use super::*;

        #[tokio::test]
        async fn forwards_request_to_device_service() {
            let mut device_mock = DeviceServiceClient::default();

            device_mock
                .expect_run_update()
                .times(1)
                .returning(|_| Box::pin(async { Ok(()) }));

            // Create RunUpdate via serde (since fields are private)
            let run_update: crate::omnect_device_service_client::RunUpdate =
                serde_json::from_str(r#"{"validate_iothub_connection": true}"#)
                    .expect("should deserialize");

            let result = FirmwareService::run_update(&device_mock, run_update).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn returns_error_on_device_service_failure() {
            let mut device_mock = DeviceServiceClient::default();

            device_mock
                .expect_run_update()
                .returning(|_| Box::pin(async { Err(anyhow::anyhow!("update execution failed")) }));

            let run_update: crate::omnect_device_service_client::RunUpdate =
                serde_json::from_str(r#"{"validate_iothub_connection": false}"#)
                    .expect("should deserialize");

            let result = FirmwareService::run_update(&device_mock, run_update).await;

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("update execution failed")
            );
        }
    }
}
