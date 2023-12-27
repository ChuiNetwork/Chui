//! This is the main file for the `bench-tps-evm` application.
//!
//! It includes the entry point of the application and the main function that orchestrates the benchmarking process.
//! The application connects to a cluster of nodes, generates and funds keypairs, and performs benchmarking of transactions per second (TPS) on the Ethereum Virtual Machine (EVM).
//! The benchmarking process involves generating and signing transactions using the keypairs, and measuring the TPS achieved.
//! The application uses the `chui_bench_tps_evm` and `chui_gossip` crates for benchmarking and network communication, respectively.
//!
//! The main function performs the following steps:
//! 1. Sets up the logger and panic hook for error handling.
//! 2. Parses command-line arguments using the `cli` module.
//! 3. Connects to the cluster of nodes specified by the entrypoint address.
//! 4. Generates and funds the required number of keypairs for benchmarking.
//! 5. Performs benchmarking of TPS using the generated keypairs.
//!
//! The benchmarking process can be customized by providing command-line arguments such as the entrypoint address, number of nodes, number of transactions, etc.
//! The application supports both single-client and multi-client benchmarking modes.
//!
//! For more details on the benchmarking process and available command-line options, refer to the `cli` module and the documentation of the `chui_bench_tps_evm` crate.
//!
//! Example usage:
//! ```shell
//! $ cargo run -- --entrypoint-addr 127.0.0.1:8000 --num-nodes 10 --tx-count 1000
//! ```
use log::*;
use std::{process::exit, sync::Arc};

use chui_bench_tps_evm::bench::generate_and_fund_keypairs;
use chui_bench_tps_evm::bench_evm::{self, Peer};
use chui_bench_tps_evm::cli;
use chui_gossip::gossip_service::{discover_cluster, get_client, get_multi_client};

use chui_streamer::socket::SocketAddrSpace;

/// Number of signatures for all transactions in ~1 week at ~100K TPS
pub const NUM_SIGNATURES_FOR_TXS: u64 = 100_000 * 60 * 60 * 24 * 7;

fn main() {
    chui_logger::setup_with_default("chui=info");
    chui_metrics::set_panic_hook("bench-tps", None);

    let matches = cli::build_args(chui_version::version!()).get_matches();
    let cli_config = cli::extract_args(&matches);

    let cli::Config {
        entrypoint_addr,
        faucet_addr,
        id,
        num_nodes,
        tx_count,
        keypair_multiplier,
        multi_client,
        num_lamports_per_account,
        target_node,
        ..
    } = &cli_config;

    let keypair_count = *tx_count * keypair_multiplier;

    info!("Connecting to the cluster");
    let nodes = discover_cluster(entrypoint_addr, *num_nodes, SocketAddrSpace::Unspecified).unwrap_or_else(|err| {
        eprintln!("Failed to discover {} nodes: {:?}", num_nodes, err);
        exit(1);
    });

    let client = if *multi_client {
        let (client, num_clients) = get_multi_client(&nodes, &SocketAddrSpace::Unspecified);
        if nodes.len() < num_clients {
            eprintln!(
                "Error: Insufficient nodes discovered.  Expecting {} or more",
                num_nodes
            );
            exit(1);
        }
        Arc::new(client)
    } else if let Some(target_node) = target_node {
        info!("Searching for target_node: {:?}", target_node);
        let mut target_client = None;
        for node in nodes {
            if node.id == *target_node {
                target_client = Some(Arc::new(get_client(&[node], &SocketAddrSpace::Unspecified)));
                break;
            }
        }
        target_client.unwrap_or_else(|| {
            eprintln!("Target node {} not found", target_node);
            exit(1);
        })
    } else {
        Arc::new(get_client(&nodes, &SocketAddrSpace::Unspecified))
    };

    let keypairs = generate_and_fund_keypairs(
        client.clone(),
        Some(*faucet_addr),
        id,
        keypair_count,
        *num_lamports_per_account,
    )
    .unwrap_or_else(|e| {
        eprintln!("Error could not fund keys: {:?}", e);
        exit(1);
    });
    let keypairs = bench_evm::generate_and_fund_evm_keypairs(
        client.clone(),
        Some(*faucet_addr),
        keypairs,
        *num_lamports_per_account,
    )
    .unwrap_or_else(|e| {
        eprintln!("Error could not fund evm keys: {:?}", e);
        exit(1);
    });

    // Init nonce = 0
    let keypairs = keypairs
        .into_iter()
        .map(|(k, s)| Peer(std::sync::Arc::new(k), s, 0))
        .collect();
    bench_evm::do_bench_tps(client, cli_config, keypairs);
}
