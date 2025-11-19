//! Firmware update service
//!
//! Handles firmware file management and operations independent of HTTP concerns.

use crate::{config::AppConfig, omnect_device_service_client::DeviceServiceClient};
use actix_multipart::form::tempfile::TempFile;
use anyhow::{Context, Result};
use log::{debug, error};
use std::{fs, os::unix::fs::PermissionsExt};

/// Service for firmware update file operations
pub struct FirmwareService;

impl FirmwareService {
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
        fs::copy(tmp_update_file, local_update_file).context("failed to copy firmware to data dir")?;

        // 3. allow host to access the file
        let metadata = fs::metadata(local_update_file).context("failed to get firmware metadata")?;
        let mut perm = metadata.permissions();
        perm.set_mode(0o750);
        fs::set_permissions(local_update_file, perm).context("failed to set firmware permissions")?;

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

    #[test]
    fn test_clear_data_folder() {
        let data_path = &AppConfig::get().paths.data_dir;

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

        // Clear folder (testing private method via module visibility)
        FirmwareService::clear_data_folder().expect("should clear folder");

        // Verify files are deleted
        assert!(!data_path.join("file1.txt").exists());
        assert!(!data_path.join("file2.txt").exists());
    }
}
