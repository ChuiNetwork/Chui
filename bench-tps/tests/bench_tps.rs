/// This code defines a test function `test_bench_tps_local_cluster_chui` that benchmarks the transactions per second (TPS) of a local Chui cluster.
/// The function creates a local cluster with a single node and a specified number of validator configurations.
/// It sets up a local faucet and transfers funds to a specified number of keypairs.
/// Then, it executes the `do_bench_tps` function to perform the TPS benchmarking.
/// The benchmarking parameters, such as the number of transactions, duration, and other configurations, can be customized through the `Config` struct.
/// This test function is annotated with `#[serial]` to ensure it runs sequentially and not in parallel with other tests.
fn test_bench_tps_local_cluster(config: Config) {
    // ... (existing code)
}

/// This test function `test_bench_tps_local_cluster_chui` is a wrapper around the `test_bench_tps_local_cluster` function.
/// It sets the default benchmarking parameters and calls the `test_bench_tps_local_cluster` function with these parameters.
#[test]
#[serial]
fn test_bench_tps_local_cluster_chui() {
    // ... (existing code)
}
#![allow(clippy::integer_arithmetic)]
use {
    serial_test::serial,
    chui_bench_tps::{
        bench::{do_bench_tps, generate_and_fund_keypairs},
        cli::Config,
    },
    chui_client::thin_client::create_client,
    chui_core::validator::ValidatorConfig,
    chui_faucet::faucet::run_local_faucet_with_port,
    chui_gossip::cluster_info::VALIDATOR_PORT_RANGE,
    chui_local_cluster::{
        local_cluster::{ClusterConfig, LocalCluster},
        validator_configs::make_identical_validator_configs,
    },
    chui_sdk::signature::{Keypair, Signer},
    chui_streamer::socket::SocketAddrSpace,
    std::{
        sync::{mpsc::channel, Arc},
        time::Duration,
    },
};

fn test_bench_tps_local_cluster(config: Config) {
    let native_instruction_processors = vec![];

    chui_logger::setup();
    const NUM_NODES: usize = 1;
    let cluster = LocalCluster::new(
        &mut ClusterConfig {
            node_stakes: vec![999_990; NUM_NODES],
            cluster_lamports: 200_000_000,
            validator_configs: make_identical_validator_configs(
                &ValidatorConfig::default_for_test(),
                NUM_NODES,
            ),
            native_instruction_processors,
            ..ClusterConfig::default()
        },
        SocketAddrSpace::Unspecified,
    );

    let faucet_keypair = Keypair::new();
    cluster.transfer(
        &cluster.funding_keypair,
        &faucet_keypair.pubkey(),
        100_000_000,
    );

    let client = Arc::new(create_client(
        (cluster.entry_point_info.rpc, cluster.entry_point_info.tpu),
        VALIDATOR_PORT_RANGE,
    ));

    let (addr_sender, addr_receiver) = channel();
    run_local_faucet_with_port(faucet_keypair, addr_sender, None, 0);
    let faucet_addr = addr_receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("run_local_faucet")
        .expect("faucet_addr");

    let lamports_per_account = 100;

    let keypair_count = config.tx_count * config.keypair_multiplier;
    let keypairs = generate_and_fund_keypairs(
        client.clone(),
        Some(faucet_addr),
        &config.id,
        keypair_count,
        lamports_per_account,
    )
    .unwrap();

    let _total = do_bench_tps(client, config, keypairs);

    #[cfg(not(debug_assertions))]
    assert!(_total > 100);
}

#[test]
#[serial]
fn test_bench_tps_local_cluster_chui() {
    test_bench_tps_local_cluster(Config {
        tx_count: 100,
        duration: Duration::from_secs(10),
        ..Config::default()
    });
}
