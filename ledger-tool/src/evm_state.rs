use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicU64,
};

use anyhow::{ensure, Result};
use clap::{value_t_or_exit, App, AppSettings, Arg, ArgMatches, SubCommand};
use log::*;
use solana_clap_utils::ArgConstant;

use evm_state::{storage::{
    inspectors::verifier::{AccountsVerifier, HashVerifier},
    inspectors::NoopInspector,
    walker::Walker,
}, StorageSecondary};
use rayon::prelude::*;

use evm_state::{
    storage::cleaner,
    storage::{inspectors, Storage},
    H256,
};
// use rayon::prelude::*;

pub trait EvmStateSubCommand {
    fn evm_state_subcommand(self) -> Self;
}

const ROOT_ARG: ArgConstant<'static> = ArgConstant {
    name: "root",
    long: "root",
    help: "EVM state root hash",
};

impl EvmStateSubCommand for App<'_, '_> {
    fn evm_state_subcommand(self) -> Self {
        self.subcommand(
            SubCommand::with_name("evm_state")
                .about("EVM state utilities")
                .arg(
                    Arg::with_name("secondary_mode")
                        .long("secondary")
                        .required(false)
                        .takes_value(false)
                        .help("whether to open evm_state_db in secondary mode"),
                )
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("purge")
                        .about("Cleanup EVM state data unreachable from state root")
                        .arg(
                            Arg::with_name(ROOT_ARG.name)
                                .long(ROOT_ARG.long)
                                .required(true)
                                .takes_value(true)
                                .help(ROOT_ARG.help),
                        )
                        .arg(
                            Arg::with_name("dry_run")
                                .long("dry-run")
                                .help("Do nothing, just collect hashes and print them"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("copy")
                        .about("Copy EVM accounts state into destination RocksDB")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::with_name(ROOT_ARG.name)
                                .long(ROOT_ARG.long)
                                .required(true)
                                .takes_value(true)
                                .help(ROOT_ARG.help),
                        )
                        .arg(
                            Arg::with_name("destination")
                                .long("destination")
                                .required(true)
                                .takes_value(true)
                                .help("Path to destination RocksDB"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("verify")
                        .about("Verify hashes of data reachable from state root")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::with_name(ROOT_ARG.name)
                                .long(ROOT_ARG.long)
                                .required(true)
                                .takes_value(true)
                                .help(ROOT_ARG.help),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("balance-lamports")
                        .about("Get balance of evm state accounts")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::with_name(ROOT_ARG.name)
                                .long(ROOT_ARG.long)
                                .required(true)
                                .takes_value(true)
                                .help(ROOT_ARG.help),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("list-roots").about("List roots in gc counter table"),
                ),
        )
    }
}

pub fn process_evm_state_command(evm_state_path: &Path, matches: &ArgMatches<'_>) -> Result<()> {
    let secondary_mode = matches.is_present("secondary_mode");
    if secondary_mode {
        let storage = Storage::open_secondary_persistent(
            evm_state_path,
            true, // enable gc
        )?;
        secondary_evm_state_command(storage, matches)?;
    } else {
        let storage = Storage::open_persistent(
            evm_state_path,
            true, // enable gc
        )?;
        primary_evm_state_command(storage, matches)?;
    }

    Ok(())
}

 fn primary_evm_state_command(storage: Storage, matches: &ArgMatches<'_>) -> Result<()> {

    match matches.subcommand() {
        ("purge", Some(matches)) => {
            let root = value_t_or_exit!(matches, ROOT_ARG.name, H256);
            let is_dry_run = matches.is_present("dry_run");

            ensure!(storage.check_root_exist(root));
            let db = storage.db();

            if is_dry_run {
                info!("Dry run, do nothing after collecting keys ...");
            }

            let trie_collector =
                std::sync::Arc::new(inspectors::memorizer::TrieCollector::default());
            let accounts = {
                let accounts_state_walker = Walker::new_sec_encoding(
                    db,
                    trie_collector.clone(),
                    inspectors::memorizer::AccountStorageRootsCollector::default(),
                );
                accounts_state_walker.traverse(root)?;
                accounts_state_walker.data_inspector.inner.summarize();

                trie_collector.summarize("Account");
                let storages_walker = Walker::new_raw(db, trie_collector.clone(), NoopInspector);
                for storage_root in accounts_state_walker
                    .data_inspector
                    .inner
                    .storage_roots
                    .iter()
                {
                    storages_walker.traverse(*storage_root)?;
                }
                trie_collector.summarize("Total");
                accounts_state_walker.data_inspector.inner
            };

            if !is_dry_run {
                let cleaner = cleaner::Cleaner::new_with(db, trie_collector, accounts);
                cleaner.cleanup()?;
            }
        }
        ("list-roots", Some(_)) => {
            storage.list_roots()?;
        }
        ("copy", Some(matches)) => {
            let root = value_t_or_exit!(matches, ROOT_ARG.name, H256);
            let destination = value_t_or_exit!(matches, "destination", PathBuf);

            assert!(storage.check_root_exist(root));
            let destination = Storage::open_persistent(
                destination,
                true, // enable gc
            )?;

            let source = storage.clone();
            let streamer = inspectors::streamer::AccountsStreamer {
                source,
                destinations: &[destination],
            };
            let walker = Walker::new_shared(storage, streamer);
            walker.traverse(root)?;
        }
        ("verify", Some(matches)) => {
            let root = value_t_or_exit!(matches, ROOT_ARG.name, H256);

            assert!(storage.check_root_exist(root));
            let db = storage.db();

            let accounts_verifier = AccountsVerifier::new(storage.clone());
            let walker = Walker::new_sec_encoding(db, HashVerifier, accounts_verifier);
            walker.traverse(root)?;

            walker
                .data_inspector
                .inner
                .storage_roots
                .into_par_iter()
                .try_for_each(|storage_root| {
                    Walker::new_raw(db, HashVerifier, NoopInspector).traverse(storage_root)
                })?
        }
        ("balance-lamports", Some(matches)) => {
            let root = value_t_or_exit!(matches, ROOT_ARG.name, H256);

            assert!(storage.check_root_exist(root));
            let db = storage.db();

            let accounts_verifier = BalanceCounter::new();
            let walker = Walker::new_sec_encoding(db, HashVerifier, accounts_verifier);
            walker.traverse(root)?;

            println!("Total balance = {:?}", walker.data_inspector.inner.balance)
        }
        unhandled => panic!("Unhandled {:?}", unhandled),
    }
    Ok(())
}

fn secondary_evm_state_command(storage: StorageSecondary, matches: &ArgMatches<'_>) -> Result<()> {

    match matches.subcommand() {
        ("list-roots", Some(_)) => {
            storage.list_roots()?;
        }
        unhandled => panic!("Unhandled {:?}", unhandled),
    }
    Ok(())
}

use evm_state::Account;

pub struct BalanceCounter {
    balance: AtomicU64,
}

impl BalanceCounter {
    pub fn new() -> Self {
        Self {
            balance: AtomicU64::default(),
        }
    }
}
use evm_state::storage::inspectors::DataInspector;
impl DataInspector<H256, Account> for BalanceCounter {
    fn inspect_data(&self, _key: H256, account: Account) -> Result<()> {
        let (lamports, _) =
            solana_evm_loader_program::scope::evm::gweis_to_lamports(account.balance);
        self.balance
            .fetch_add(lamports, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
