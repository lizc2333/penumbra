/*
    SPDX-License-Identifier: AGPL-3.0-or-later
    SPDX-FileCopyrightText: 2025 Shomy
*/
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use log::info;
use penumbra::Device;
use penumbra::core::storage::Partition;
use tokio::fs::File;
use tokio::io::BufWriter;

use crate::cli::MtkCommand;
use crate::cli::common::{CONN_DA, CommandMetadata, DaArgs};
use crate::cli::helpers::AntumbraProgress;
use crate::cli::state::PersistedDeviceState;

#[derive(Args, Debug)]
pub struct UploadArgs {
    #[command(flatten)]
    pub da: DaArgs,
    /// The partition to read
    pub partition: String,
    /// The destination file
    pub output_file: PathBuf,
}

impl CommandMetadata for UploadArgs {
    fn visible_aliases() -> &'static [&'static str] {
        &["up"]
    }

    fn about() -> &'static str {
        "Upload a partition from the device to the host."
    }

    fn long_about() -> &'static str {
        "Upload (readback) a specificed partition on the device to a file on the host.
        Use this command for reading back if the `read` command fails."
    }
}

#[async_trait]
impl MtkCommand for UploadArgs {
    async fn run(&self, dev: &mut Device, state: &mut PersistedDeviceState) -> Result<()> {
        dev.enter_da_mode().await?;

        state.connection_type = CONN_DA;
        state.flash_mode = 1;

        let storage = dev
            .dev_info
            .storage()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get storage information from device."))?;

        let pl_part = storage.get_pl_part1();

        let preloader = Partition {
            name: "preloader".to_string(),
            size: 0x400000, // 4MB,
            address: 0,
            kind: pl_part,
        };

        let mut partitions = dev.get_partitions().await;
        partitions.push(preloader);
        dev.dev_info.set_partitions(partitions).await;

        let partitions = dev.dev_info.get_partition(&self.partition).await;

        let partition = match partitions {
            Some(p) => p,
            None => {
                info!("Partition '{}' not found on device.", self.partition);
                return Err(anyhow::anyhow!("Partition '{}' not found on device.", self.partition));
            }
        };

        let total_size = partition.size as u64;
        let pb = AntumbraProgress::new(total_size);

        let mut progress_callback = {
            let pb = &pb;
            move |written: usize, total: usize| {
                pb.update(written as u64, "Uploading...");

                if written >= total {
                    pb.finish("Upload complete!");
                }
            }
        };

        let file = File::create(&self.output_file).await?;
        let mut writer = BufWriter::new(file);

        match dev.upload(&self.partition, &mut writer, &mut progress_callback).await {
            Ok(_) => {}
            Err(e) => {
                pb.abandon("Upload failed!");
                return Err(e)?;
            }
        };

        Ok(())
    }

    fn da(&self) -> Option<&PathBuf> {
        Some(&self.da.da_file)
    }

    fn pl(&self) -> Option<&PathBuf> {
        self.da.preloader_file.as_ref()
    }
}
