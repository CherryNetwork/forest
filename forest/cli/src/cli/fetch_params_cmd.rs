// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::cli::Config;
use structopt::StructOpt;

use forest_paramfetch::{get_params_default, SectorSizeOpt};
use fvm_shared::sector::SectorSize;

use super::cli_error_and_die;

#[allow(missing_docs)]
#[derive(Debug, StructOpt)]
pub struct FetchCommands {
    /// Download all proof parameters
    #[structopt(short, long)]
    all: bool,
    /// Download only verification keys
    #[structopt(short, long)]
    keys: bool,
    /// Size in bytes
    #[structopt(required_ifs(&[("all", "false"), ("keys", "false")]))]
    params_size: Option<String>,
}

impl FetchCommands {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let sizes = if self.all {
            SectorSizeOpt::All
        } else if let Some(size) = &self.params_size {
            let sector_size = ram_to_int(size)?;
            SectorSizeOpt::Size(sector_size)
        } else if self.keys {
            SectorSizeOpt::Keys
        } else {
            cli_error_and_die(
                "Sector size option must be chosen. Choose between --all, --keys, or <size>",
                1,
            );
        };

        get_params_default(&config.client.data_dir, sizes).await
    }
}

/// Converts a human readable string to a `u64` size.
fn ram_to_int(size: &str) -> anyhow::Result<SectorSize> {
    // * there is no library to do this, but if other sector sizes are supported in future
    // this should probably be changed to parse from string to `SectorSize`
    let mut trimmed = size.trim_end_matches('B');
    trimmed = trimmed.trim_end_matches('b');

    match trimmed {
        "2048" | "2Ki" | "2ki" => Ok(SectorSize::_2KiB),
        "8388608" | "8Mi" | "8mi" => Ok(SectorSize::_8MiB),
        "536870912" | "512Mi" | "512mi" => Ok(SectorSize::_512MiB),
        "34359738368" | "32Gi" | "32gi" => Ok(SectorSize::_32GiB),
        "68719476736" | "64Gi" | "64gi" => Ok(SectorSize::_64GiB),
        _ => Err(anyhow::Error::msg(format!(
            "Failed to parse: {size}. Must be a valid sector size"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_str_conversions() {
        assert_eq!(ram_to_int("2048").unwrap(), SectorSize::_2KiB);
        assert_eq!(ram_to_int("2048B").unwrap(), SectorSize::_2KiB);
        assert_eq!(ram_to_int("2kib").unwrap(), SectorSize::_2KiB);
        assert_eq!(ram_to_int("8Mib").unwrap(), SectorSize::_8MiB);
        assert_eq!(ram_to_int("512MiB").unwrap(), SectorSize::_512MiB);
        assert_eq!(ram_to_int("32Gi").unwrap(), SectorSize::_32GiB);
        assert_eq!(ram_to_int("32GiB").unwrap(), SectorSize::_32GiB);
        assert_eq!(ram_to_int("64Gib").unwrap(), SectorSize::_64GiB);
    }
}
