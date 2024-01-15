use log::*;
use rayon::prelude::*;
use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    process::exit,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
    thread::{sleep, Builder, JoinHandle},
    time::{Duration, Instant},
};

use solana_client::perf_utils::{sample_txs, SampleStats};
use solana_core::gen_keys::GenKeys;
use solana_faucet::faucet::request_airdrop_transaction;
use solana_measure::measure::Measure;
use solana_metrics::{self, datapoint_info};
use solana_sdk::{
    client::Client,
    clock::{DEFAULT_MS_PER_SLOT, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, MAX_PROCESSING_AGE},
    commitment_config::CommitmentConfig,
    hash::Hash,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    timing::duration_as_s,
    transaction::Transaction, instruction::{AccountMeta, Instruction},
};

// The point at which transactions become "too old", in seconds.
pub const MAX_TX_QUEUE_AGE: u64 =
    MAX_PROCESSING_AGE as u64 * DEFAULT_TICKS_PER_SLOT / DEFAULT_TICKS_PER_SECOND;

pub const MAX_SPENDS_PER_TX: u64 = 4;

#[derive(Debug)]
pub enum BenchTpsError {
    AirdropFailure,
}

pub type Result<T> = std::result::Result<T, BenchTpsError>;

pub type SharedTransactions = Arc<RwLock<VecDeque<Vec<(Transaction, u64)>>>>;

pub fn get_recent_blockhash<T: Client>(client: &T) -> Hash {
    loop {
        match client.get_latest_blockhash_with_commitment(CommitmentConfig::processed()) {
            Ok((blockhash, _last_valid_height)) => {
                return blockhash
            }
            Err(err) => {
                info!("Couldn't get recent blockhash: {:?}", err);
                if cfg!(not(test)) {
                    sleep(Duration::from_secs(1));
                }
            }
        };
    }
}

pub fn wait_for_target_slots_per_epoch<T>(target_slots_per_epoch: u64, client: &Arc<T>)
where
    T: 'static + Client,
{
    if target_slots_per_epoch != 0 {
        info!(
            "Waiting until epochs are {} slots long..",
            target_slots_per_epoch
        );
        loop {
            if let Ok(epoch_info) = client.get_epoch_info() {
                if epoch_info.slots_in_epoch >= target_slots_per_epoch {
                    info!("Done epoch_info: {:?}", epoch_info);
                    return;
                }
                info!(
                    "Waiting for epoch: {} now: {}",
                    target_slots_per_epoch, epoch_info.slots_in_epoch
                );
            }
            sleep(Duration::from_secs(3));
        }
    }
}

pub fn create_sampler_thread<T>(
    client: &Arc<T>,
    exit_signal: &Arc<AtomicBool>,
    sample_period: u64,
    maxes: &Arc<RwLock<Vec<(String, SampleStats)>>>,
) -> JoinHandle<()>
where
    T: 'static + Client + Send + Sync,
{
    info!("Sampling TPS every {} second...", sample_period);
    let exit_signal = exit_signal.clone();
    let maxes = maxes.clone();
    let client = client.clone();
    Builder::new()
        .name("solana-client-sample".to_string())
        .spawn(move || {
            sample_txs(&exit_signal, &maxes, sample_period, &client);
        })
        .unwrap()
}

pub fn metrics_submit_lamport_balance(lamport_balance: u64) {
    info!("Token balance: {}", lamport_balance);
    datapoint_info!(
        "bench-tps-lamport_balance",
        ("balance", lamport_balance, i64)
    );
}

fn get_new_latest_blockhash<T: Client>(client: &Arc<T>, blockhash: &Hash) -> Option<Hash> {
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        if let Ok(new_blockhash) = client.get_latest_blockhash() {
            if new_blockhash != *blockhash {
                return Some(new_blockhash);
            }
        }
        debug!("Got same blockhash ({:?}), will retry...", blockhash);

        // Retry ~twice during a slot
        sleep(Duration::from_millis(DEFAULT_MS_PER_SLOT / 2));
    }
    None
}

pub fn poll_blockhash<T: Client>(
    exit_signal: &Arc<AtomicBool>,
    blockhash: &Arc<RwLock<Hash>>,
    client: &Arc<T>,
    id: &Pubkey,
) {
    let mut blockhash_last_updated = Instant::now();
    let mut last_error_log = Instant::now();
    loop {
        let blockhash_updated = {
            let old_blockhash = *blockhash.read().unwrap();
            if let Some(new_blockhash) = get_new_latest_blockhash(client, &old_blockhash) {
                *blockhash.write().unwrap() = new_blockhash;
                blockhash_last_updated = Instant::now();
                true
            } else {
                if blockhash_last_updated.elapsed().as_secs() > 120 {
                    eprintln!("Blockhash is stuck");
                    exit(1)
                } else if blockhash_last_updated.elapsed().as_secs() > 30
                    && last_error_log.elapsed().as_secs() >= 1
                {
                    last_error_log = Instant::now();
                    error!("Blockhash is not updating");
                }
                false
            }
        };

        if blockhash_updated {
            let balance = client.get_balance(id).unwrap_or(0);
            metrics_submit_lamport_balance(balance);
        }

        if exit_signal.load(Ordering::Relaxed) {
            break;
        }

        sleep(Duration::from_millis(50));
    }
}

fn verify_funding_transfer<T: Client>(client: &Arc<T>, tx: &Transaction, amount: u64) -> bool {
    for a in &tx.message().account_keys[1..] {
        match client.get_balance_with_commitment(a, CommitmentConfig::processed()) {
            Ok(balance) => return balance >= amount,
            Err(err) => error!("failed to get balance {:?}", err),
        }
    }
    false
}

trait FundingTransactions<'a> {
    fn fund<T: 'static + Client + Send + Sync>(
        &mut self,
        client: &Arc<T>,
        to_fund: &[(&'a Keypair, Vec<(Pubkey, u64)>)],
        to_lamports: u64,
    );
    fn make(&mut self, to_fund: &[(&'a Keypair, Vec<(Pubkey, u64)>)]);
    fn sign(&mut self, blockhash: Hash);
    fn send<T: Client>(&self, client: &Arc<T>);
    fn verify<T: 'static + Client + Send + Sync>(&mut self, client: &Arc<T>, to_lamports: u64);
}

impl<'a> FundingTransactions<'a> for Vec<(&'a Keypair, Transaction)> {
    fn fund<T: 'static + Client + Send + Sync>(
        &mut self,
        client: &Arc<T>,
        to_fund: &[(&'a Keypair, Vec<(Pubkey, u64)>)],
        to_lamports: u64,
    ) {
        self.make(to_fund);

        let mut tries = 0;
        while !self.is_empty() {
            info!(
                "{} {} each to {} accounts in {} txs",
                if tries == 0 {
                    "transferring"
                } else {
                    " retrying"
                },
                to_lamports,
                self.len() * MAX_SPENDS_PER_TX as usize,
                self.len(),
            );

            let blockhash = get_recent_blockhash(client.as_ref());

            // re-sign retained to_fund_txes with updated blockhash
            self.sign(blockhash);
            self.send(client);

            // Sleep a few slots to allow transactions to process
            sleep(Duration::from_secs(1));

            self.verify(client, to_lamports);

            // retry anything that seems to have dropped through cracks
            //  again since these txs are all or nothing, they're fine to
            //  retry
            tries += 1;
        }
        info!("transferred");
    }

    fn make(&mut self, to_fund: &[(&'a Keypair, Vec<(Pubkey, u64)>)]) {
        let mut make_txs = Measure::start("make_txs");
        let to_fund_txs: Vec<(&Keypair, Transaction)> = to_fund
            .par_iter()
            .map(|(k, t)| {
                let instructions = system_instruction::transfer_many(&k.pubkey(), t);
                let message = Message::new(&instructions, Some(&k.pubkey()));
                (*k, Transaction::new_unsigned(message))
            })
            .collect();
        make_txs.stop();
        debug!(
            "make {} unsigned txs: {}us",
            to_fund_txs.len(),
            make_txs.as_us()
        );
        self.extend(to_fund_txs);
    }

    fn sign(&mut self, blockhash: Hash) {
        let mut sign_txs = Measure::start("sign_txs");
        self.par_iter_mut().for_each(|(k, tx)| {
            tx.sign(&[*k], blockhash);
        });
        sign_txs.stop();
        debug!("sign {} txs: {}us", self.len(), sign_txs.as_us());
    }

    fn send<T: Client>(&self, client: &Arc<T>) {
        let mut send_txs = Measure::start("send_txs");
        self.iter().for_each(|(_, tx)| {
            client.async_send_transaction(tx.clone()).expect("transfer");
        });
        send_txs.stop();
        debug!("send {} txs: {}us", self.len(), send_txs.as_us());
    }

    fn verify<T: 'static + Client + Send + Sync>(&mut self, client: &Arc<T>, to_lamports: u64) {
        let starting_txs = self.len();
        let verified_txs = Arc::new(AtomicUsize::new(0));
        let too_many_failures = Arc::new(AtomicBool::new(false));
        let loops = if starting_txs < 1000 { 3 } else { 1 };
        // Only loop multiple times for small (quick) transaction batches
        let time = Arc::new(Mutex::new(Instant::now()));
        for _ in 0..loops {
            let time = time.clone();
            let failed_verify = Arc::new(AtomicUsize::new(0));
            let client = client.clone();
            let verified_txs = &verified_txs;
            let failed_verify = &failed_verify;
            let too_many_failures = &too_many_failures;
            let verified_set: HashSet<Pubkey> = self
                .par_iter()
                .filter_map(move |(k, tx)| {
                    if too_many_failures.load(Ordering::Relaxed) {
                        return None;
                    }

                    let verified = if verify_funding_transfer(&client, tx, to_lamports) {
                        verified_txs.fetch_add(1, Ordering::Relaxed);
                        Some(k.pubkey())
                    } else {
                        failed_verify.fetch_add(1, Ordering::Relaxed);
                        None
                    };

                    let verified_txs = verified_txs.load(Ordering::Relaxed);
                    let failed_verify = failed_verify.load(Ordering::Relaxed);
                    let remaining_count = starting_txs.saturating_sub(verified_txs + failed_verify);
                    if failed_verify > 100 && failed_verify > verified_txs {
                        too_many_failures.store(true, Ordering::Relaxed);
                        warn!(
                            "Too many failed transfers... {} remaining, {} verified, {} failures",
                            remaining_count, verified_txs, failed_verify
                        );
                    }
                    if remaining_count > 0 {
                        let mut time_l = time.lock().unwrap();
                        if time_l.elapsed().as_secs() > 2 {
                            info!(
                                "Verifying transfers... {} remaining, {} verified, {} failures",
                                remaining_count, verified_txs, failed_verify
                            );
                            *time_l = Instant::now();
                        }
                    }

                    verified
                })
                .collect();

            self.retain(|(k, _)| !verified_set.contains(&k.pubkey()));
            if self.is_empty() {
                break;
            }
            info!("Looping verifications");

            let verified_txs = verified_txs.load(Ordering::Relaxed);
            let failed_verify = failed_verify.load(Ordering::Relaxed);
            let remaining_count = starting_txs.saturating_sub(verified_txs + failed_verify);
            info!(
                "Verifying transfers... {} remaining, {} verified, {} failures",
                remaining_count, verified_txs, failed_verify
            );
            sleep(Duration::from_millis(100));
        }
    }
}

/// fund the dests keys by spending all of the source keys into MAX_SPENDS_PER_TX
/// on every iteration.  This allows us to replay the transfers because the source is either empty,
/// or full
pub fn fund_keys<T: 'static + Client + Send + Sync>(
    client: Arc<T>,
    source: &Keypair,
    dests: &[Keypair],
    total: u64,
    max_fee: u64,
    lamports_per_account: u64,
) {
    let mut funded: Vec<&Keypair> = vec![source];
    let mut funded_funds = total;
    let mut not_funded: Vec<&Keypair> = dests.iter().collect();
    while !not_funded.is_empty() {
        // Build to fund list and prepare funding sources for next iteration
        let mut new_funded: Vec<&Keypair> = vec![];
        let mut to_fund: Vec<(&Keypair, Vec<(Pubkey, u64)>)> = vec![];
        let to_lamports = (funded_funds - lamports_per_account - max_fee) / MAX_SPENDS_PER_TX;
        for f in funded {
            let start = not_funded.len() - MAX_SPENDS_PER_TX as usize;
            let dests: Vec<_> = not_funded.drain(start..).collect();
            let spends: Vec<_> = dests.iter().map(|k| (k.pubkey(), to_lamports)).collect();
            to_fund.push((f, spends));
            new_funded.extend(dests.into_iter());
        }

        // try to transfer a "few" at a time with recent blockhash
        //  assume 4MB network buffers, and 512 byte packets
        const FUND_CHUNK_LEN: usize = 4 * 1024 * 1024 / 512;

        to_fund.chunks(FUND_CHUNK_LEN).for_each(|chunk| {
            Vec::<(&Keypair, Transaction)>::with_capacity(chunk.len()).fund(
                &client,
                chunk,
                to_lamports,
            );
        });

        info!("funded: {} left: {}", new_funded.len(), not_funded.len());
        funded = new_funded;
        funded_funds = to_lamports;
    }
}

pub fn airdrop_lamports<T: Client>(
    client: &T,
    faucet_addr: &SocketAddr,
    id: &Keypair,
    desired_balance: u64,
) -> Result<()> {
    let starting_balance = client.get_balance(&id.pubkey()).unwrap_or(0);
    metrics_submit_lamport_balance(starting_balance);
    info!("starting balance {}", starting_balance);

    if starting_balance < desired_balance {
        let airdrop_amount = desired_balance - starting_balance;
        info!(
            "Airdropping {:?} lamports from {} for {}",
            airdrop_amount,
            faucet_addr,
            id.pubkey(),
        );

        let blockhash = get_recent_blockhash(client);
        match request_airdrop_transaction(faucet_addr, &id.pubkey(), airdrop_amount, blockhash) {
            Ok(transaction) => {
                let mut tries = 0;
                loop {
                    tries += 1;
                    let signature = client.async_send_transaction(transaction.clone()).unwrap();
                    let result = client.poll_for_signature_confirmation(&signature, 1);

                    if result.is_ok() {
                        break;
                    }
                    if tries >= 5 {
                        panic!(
                            "Error requesting airdrop: to addr: {:?} amount: {} {:?}",
                            faucet_addr, airdrop_amount, result
                        )
                    }
                }
            }
            Err(err) => {
                panic!(
                    "Error requesting airdrop: {:?} to addr: {:?} amount: {}",
                    err, faucet_addr, airdrop_amount
                );
            }
        };

        let current_balance = client
            .get_balance_with_commitment(&id.pubkey(), CommitmentConfig::processed())
            .unwrap_or_else(|e| {
                info!("airdrop error {}", e);
                starting_balance
            });
        info!("current balance {}...", current_balance);

        metrics_submit_lamport_balance(current_balance);
        if current_balance - starting_balance != airdrop_amount {
            info!(
                "Airdrop failed! {} {} {}",
                id.pubkey(),
                current_balance,
                starting_balance
            );
            return Err(BenchTpsError::AirdropFailure);
        }
    }
    Ok(())
}

pub fn compute_and_report_stats(
    maxes: &Arc<RwLock<Vec<(String, SampleStats)>>>,
    sample_period: u64,
    tx_send_elapsed: &Duration,
    total_tx_send_count: usize,
) {
    // Compute/report stats
    let mut max_of_maxes = 0.0;
    let mut max_tx_count = 0;
    let mut nodes_with_zero_tps = 0;
    let mut total_maxes = 0.0;
    info!(" Node address        |       Max TPS | Total Transactions");
    info!("---------------------+---------------+--------------------");

    for (sock, stats) in maxes.read().unwrap().iter() {
        let maybe_flag = match stats.txs {
            0 => "!!!!!",
            _ => "",
        };

        info!(
            "{:20} | {:13.2} | {} {}",
            sock, stats.tps, stats.txs, maybe_flag
        );

        if stats.tps == 0.0 {
            nodes_with_zero_tps += 1;
        }
        total_maxes += stats.tps;

        if stats.tps > max_of_maxes {
            max_of_maxes = stats.tps;
        }
        if stats.txs > max_tx_count {
            max_tx_count = stats.txs;
        }
    }

    if total_maxes > 0.0 {
        let num_nodes_with_tps = maxes.read().unwrap().len() - nodes_with_zero_tps;
        let average_max = total_maxes / num_nodes_with_tps as f32;
        info!(
            "\nAverage max TPS: {:.2}, {} nodes had 0 TPS",
            average_max, nodes_with_zero_tps
        );
    }

    let total_tx_send_count = total_tx_send_count as u64;
    let drop_rate = if total_tx_send_count > max_tx_count {
        (total_tx_send_count - max_tx_count) as f64 / total_tx_send_count as f64
    } else {
        0.0
    };
    info!(
        "\nHighest TPS: {:.2} sampling period {}s max transactions: {} clients: {} drop rate: {:.2}",
        max_of_maxes,
        sample_period,
        max_tx_count,
        maxes.read().unwrap().len(),
        drop_rate,
    );
    info!(
        "\tAverage TPS: {}",
        max_tx_count as f32 / duration_as_s(tx_send_elapsed)
    );
}

pub fn generate_keypairs(seed_keypair: &Keypair, count: u64) -> (Vec<Keypair>, u64) {
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_keypair.to_bytes()[..32]);
    let mut rnd = GenKeys::new(seed);

    let mut total_keys = 0;
    let mut extra = 1; // This variable tracks the number of keypairs needing extra transaction fees funded
    let mut delta = 1;
    while total_keys < count {
        extra += delta;
        delta *= MAX_SPENDS_PER_TX;
        total_keys += delta;
    }
    (rnd.gen_n_keypairs(total_keys), extra)
}

pub fn generate_and_fund_keypairs<T: 'static + Client + Send + Sync>(
    client: Arc<T>,
    faucet_addr: Option<SocketAddr>,
    funding_key: &Keypair,
    keypair_count: usize,
    lamports_per_account: u64,
) -> Result<Vec<Keypair>> {
    info!("Creating {} keypairs...", keypair_count);
    let (mut keypairs, extra) = generate_keypairs(funding_key, keypair_count as u64);
    info!("Get lamports...");

    // Sample the first keypair, to prevent lamport loss on repeated solana-bench-tps executions
    let first_key = keypairs[0].pubkey();
    let first_keypair_balance = client.get_balance(&first_key).unwrap_or(0);

    // Sample the last keypair, to check if funding was already completed
    let last_key = keypairs[keypair_count - 1].pubkey();
    let last_keypair_balance = client.get_balance(&last_key).unwrap_or(0);

    // Repeated runs will eat up keypair balances from transaction fees. In order to quickly
    //   start another bench-tps run without re-funding all of the keypairs, check if the
    //   keypairs still have at least 80% of the expected funds. That should be enough to
    //   pay for the transaction fees in a new run.
    let enough_lamports = 8 * lamports_per_account / 10;
    if first_keypair_balance < enough_lamports || last_keypair_balance < enough_lamports {
        let single_sig_message = Message::new_with_blockhash(
            &[Instruction::new_with_bytes(
                Pubkey::new_unique(),
                &[],
                vec![AccountMeta::new(Pubkey::new_unique(), true)],
            )],
            None,
            &client.get_latest_blockhash().unwrap(),
        );
        let max_fee = client.get_fee_for_message(&single_sig_message).unwrap();
        let extra_fees = extra * max_fee;
        let total_keypairs = keypairs.len() as u64 + 1; // Add one for funding keypair

        let total = lamports_per_account * total_keypairs + extra_fees;
        let total = total * 2;
        let funding_key_balance = client.get_balance(&funding_key.pubkey()).unwrap_or(0);
        info!(
            "Funding keypair balance: {} max_fee: {} lamports_per_account: {} extra: {} total: {}",
            funding_key_balance, max_fee, lamports_per_account, extra, total
        );

        if client.get_balance(&funding_key.pubkey()).unwrap_or(0) < total {
            airdrop_lamports(client.as_ref(), &faucet_addr.unwrap(), funding_key, total)?;
        }

        fund_keys(
            client,
            funding_key,
            &keypairs,
            total,
            max_fee,
            lamports_per_account * 2,
        );
    }

    // 'generate_keypairs' generates extra keys to be able to have size-aligned funding batches for fund_keys.
    keypairs.truncate(keypair_count);

    Ok(keypairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench_evm::{do_bench_tps, Peer};
    use crate::cli::Config;
    use solana_evm_loader_program::scope::evm::FromKey;
    use solana_runtime::bank::Bank;
    use solana_runtime::bank_client::BankClient;
    use solana_sdk::client::SyncClient;
    use solana_sdk::fee_calculator::FeeRateGovernor;
    use solana_sdk::genesis_config::create_genesis_config;

    #[test]
    #[ignore]
    fn test_bench_tps_bank_client() {
        let (genesis_config, id) = create_genesis_config(10_000);
        let bank = Bank::new_for_tests(&genesis_config);
        let client = Arc::new(BankClient::new(bank));

        let config = Config {
            id,
            tx_count: 10,
            duration: Duration::from_secs(5),
            ..Config::default()
        };

        let keypair_count = config.tx_count * config.keypair_multiplier;
        let keypairs =
            generate_and_fund_keypairs(client.clone(), None, &config.id, keypair_count, 20)
                .unwrap();

        let keypairs =
            crate::bench_evm::generate_and_fund_evm_keypairs(client.clone(), None, keypairs, 20)
                .unwrap();
        let keypairs = keypairs
            .into_iter()
            .map(|(k, s)| Peer(std::sync::Arc::new(k), s, 0))
            .collect();
        do_bench_tps(client, config, keypairs);
    }
    #[test]
    #[ignore]
    fn test_bench_tps_fund_keys_with_fees() {
        let (mut genesis_config, id) = create_genesis_config(10_000);
        let fee_rate_governor = FeeRateGovernor::new(11, 0);
        genesis_config.fee_rate_governor = fee_rate_governor;
        let bank = Bank::new_for_tests(&genesis_config);
        let client = Arc::new(BankClient::new(bank));
        let keypair_count = 20;
        let lamports = 20;

        let keypairs =
            generate_and_fund_keypairs(client.clone(), None, &id, keypair_count, lamports).unwrap();
        assert_eq!(keypairs.len(), keypair_count);
        let keypairs = crate::bench_evm::generate_and_fund_evm_keypairs(
            client.clone(),
            None,
            keypairs,
            lamports,
        )
        .unwrap();

        for (_kp, secret_key) in &keypairs {
            assert_eq!(
                client.get_evm_balance(&secret_key.to_address()).unwrap(),
                solana_evm_loader_program::scope::evm::lamports_to_gwei(lamports)
            );
        }
    }
}
