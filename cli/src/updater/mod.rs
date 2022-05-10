mod export;
mod generate;
mod github;
mod metadata;
pub mod source;
mod wasm;

use crate::config::AppConfig;
use crate::lib::read::{hex_to_bytes, metadata_qr_in_dir, specs_qr_in_dir};
use anyhow::{anyhow, bail, ensure};
use definitions::metadata::{convert_wasm_into_metadata, MetaValues};
use log::info;
use octocrab::{models, params};

use crate::updater::export::MetaSpecs;
use definitions::network_specs::NetworkSpecsToSend;
use std::path::Path;

use crate::updater::generate::{generate_metadata_qr, generate_spec_qr};
use crate::updater::github::fetch_release_runtimes;
use crate::updater::metadata::fetch_chain_info;
use crate::updater::wasm::meta_values_from_wasm;

pub fn update_from_node(config: AppConfig) -> anyhow::Result<()> {
    let metadata_qrs = metadata_qr_in_dir(&config.qr_dir)?;
    let specs_qrs = specs_qr_in_dir(&config.qr_dir)?;

    let mut is_changed = false;
    for chain in config.chains {
        let meta_specs = fetch_chain_info(&chain.rpc_endpoint)?;
        if !specs_qrs.contains_key(chain.name.as_str()) {
            generate_spec_qr(&meta_specs, &config.qr_dir)?;
            is_changed = true;
        }
        match metadata_qrs.get(chain.name.as_str()) {
            Some((_, version)) if *version >= meta_specs.meta_values.version => (),
            _ => {
                generate_metadata_qr(
                    &meta_specs.meta_values,
                    meta_specs.specs.genesis_hash,
                    &config.qr_dir,
                )?;
                is_changed = true;
            }
        };
    }

    if !is_changed {
        println!("Everything is up to date!");
        return Ok(());
    }

    println!("Done!");
    Ok(())
}

#[tokio::main]
pub async fn update_from_github(config: AppConfig) -> anyhow::Result<()> {
    let metadata_qrs = metadata_qr_in_dir(&config.qr_dir)?;
    let runtimes = fetch_release_runtimes().await?;
    for chain in config.chains {
        if !runtimes.contains_key(&chain.name) {
            info!("no releases for {} found", chain.name);
            continue;
        }
        let wasm = runtimes.get(&chain.name).unwrap();

        let genesis_hash = chain.genesis_hash.expect(&format!(
            "cannot find genesis_hash for {} in config.toml",
            chain.name
        ));
        let genesis_hash = hex_to_bytes(&genesis_hash)?.try_into().unwrap();

        match metadata_qrs.get(&chain.name) {
            Some((_, version)) if *version >= wasm.version => (),
            _ => {
                let meta_values = meta_values_from_wasm(wasm.to_owned()).await?;
                generate_metadata_qr(&meta_values, genesis_hash, &config.qr_dir)?;
            }
        };
    }
    Ok(())
}
