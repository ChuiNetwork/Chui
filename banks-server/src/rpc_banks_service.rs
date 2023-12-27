//! The `rpc_banks_service` module implements the Chui Banks RPC API.
//!
//! This module provides the `RpcBanksService` struct, which represents the RPC service for the Chui Banks.
//! It includes methods for starting and stopping the TCP server, as well as joining the server thread.
//!
//! # Example
//!
//! ```rust
//! use std::net::SocketAddr;
//! use chui_runtime::{bank_forks::BankForks, commitment::BlockCommitmentCache};
//! use std::sync::{atomic::AtomicBool, Arc, RwLock};
//! use std::thread;
//! use tokio::runtime::Runtime;
//!
//! let listen_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
//! let tpu_addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
//! let bank_forks = Arc::new(RwLock::new(BankForks::new()));
//! let block_commitment_cache = Arc::new(RwLock::new(BlockCommitmentCache::new()));
//! let exit = Arc::new(AtomicBool::new(false));
//!
//! let rpc_service = RpcBanksService::new(
//!     listen_addr,
//!     tpu_addr,
//!     &bank_forks,
//!     &block_commitment_cache,
//!     &exit,
//! );
//!
//! // Start the TCP server
//! thread::spawn(move || {
//!     rpc_service.run();
//! });
//!
//! // ... Perform other operations ...
//!
//! // Stop the TCP server
//! exit.store(true, Ordering::Relaxed);
//! rpc_service.join().unwrap();
//! ```
//!
//! For more information, see the [Chui Banks RPC API documentation](https://docs.chui.com/rpc/banks).
//! The `rpc_banks_service` module implements the Chui Banks RPC API.

use {
    crate::banks_server::start_tcp_server,
    futures::{future::FutureExt, pin_mut, prelude::stream::StreamExt, select},
    chui_runtime::{bank_forks::BankForks, commitment::BlockCommitmentCache},
    std::{
        net::SocketAddr,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, RwLock,
        },
        thread::{self, Builder, JoinHandle},
    },
    tokio::{
        runtime::Runtime,
        time::{self, Duration},
    },
    tokio_stream::wrappers::IntervalStream,
};

pub struct RpcBanksService {
    thread_hdl: JoinHandle<()>,
}

/// Run the TCP service until `exit` is set to true
async fn start_abortable_tcp_server(
    listen_addr: SocketAddr,
    tpu_addr: SocketAddr,
    bank_forks: Arc<RwLock<BankForks>>,
    block_commitment_cache: Arc<RwLock<BlockCommitmentCache>>,
    exit: Arc<AtomicBool>,
) {
    let server = start_tcp_server(
        listen_addr,
        tpu_addr,
        bank_forks.clone(),
        block_commitment_cache.clone(),
    )
    .fuse();
    let interval = IntervalStream::new(time::interval(Duration::from_millis(100))).fuse();
    pin_mut!(server, interval);
    loop {
        select! {
            _ = server => {},
            _ = interval.select_next_some() => {
                if exit.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }
}

impl RpcBanksService {
    fn run(
        listen_addr: SocketAddr,
        tpu_addr: SocketAddr,
        bank_forks: Arc<RwLock<BankForks>>,
        block_commitment_cache: Arc<RwLock<BlockCommitmentCache>>,
        exit: Arc<AtomicBool>,
    ) {
        let server = start_abortable_tcp_server(
            listen_addr,
            tpu_addr,
            bank_forks,
            block_commitment_cache,
            exit,
        );
        Runtime::new().unwrap().block_on(server);
    }

    pub fn new(
        listen_addr: SocketAddr,
        tpu_addr: SocketAddr,
        bank_forks: &Arc<RwLock<BankForks>>,
        block_commitment_cache: &Arc<RwLock<BlockCommitmentCache>>,
        exit: &Arc<AtomicBool>,
    ) -> Self {
        let bank_forks = bank_forks.clone();
        let block_commitment_cache = block_commitment_cache.clone();
        let exit = exit.clone();
        let thread_hdl = Builder::new()
            .name("chui-rpc-banks".to_string())
            .spawn(move || {
                Self::run(
                    listen_addr,
                    tpu_addr,
                    bank_forks,
                    block_commitment_cache,
                    exit,
                )
            })
            .unwrap();

        Self { thread_hdl }
    }

    pub fn join(self) -> thread::Result<()> {
        self.thread_hdl.join()
    }
}

#[cfg(test)]
mod tests {
    use {super::*, chui_runtime::bank::Bank};

    #[test]
    fn test_rpc_banks_server_exit() {
        let bank_forks = Arc::new(RwLock::new(BankForks::new(Bank::default_for_tests())));
        let block_commitment_cache = Arc::new(RwLock::new(BlockCommitmentCache::default()));
        let exit = Arc::new(AtomicBool::new(false));
        let addr = "127.0.0.1:0".parse().unwrap();
        let service = RpcBanksService::new(addr, addr, &bank_forks, &block_commitment_cache, &exit);
        exit.store(true, Ordering::Relaxed);
        service.join().unwrap();
    }
}
