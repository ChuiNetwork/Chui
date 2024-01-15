![logo](https://github.com/ChuiNetwork/.github/assets/56628755/59313d24-8ab2-4383-9a84-9e5449a58844)

# Chui Network - Overview

## Technical Specifications

- **Token Name**: Chui (CUI)
- **Ticker**: CUI
- **Max Supply**: 200M
- **Block Time**: Approx. TBDms
- **Consensus**: Delegated Proof-of-Stake (DPoS)
- **Network Ports**: P2P (TBD), RPC (8899)
- **Chain IDs**: Mainnet (TBA), Testnet (TBA)
- **EVM Compatibility**: Full support for Ethereum Virtual Machine

## Essential Resources

- **Website**: [chui.network](https://chui.network)
- **Explorers**: [Mainnet](https://evm.chuiscan.io), [Testnet](https://testnet.chuiscan.io)
- **RPC Endpoints**: [Mainnet](https://rpc-main-1.chui.network), [Testnet](https://rpc-test-1.chui.network)
- **Socials**: [Twitter](https://twitter.com/chuinetwork), [Telegram](https://t.me/ChuiNetwork)
- **Testnet Faucet**: [Access Here](https://evm-faucet.chui.network)

## Core Features

- Scalable modular design.
- Full Ethereum compatibility via JSON-RPC.
- Solidity/Vyper development with EVM integration.
- Enhanced cross-chain functionality.
- Advanced Runtime plugins extending Ethereum's capabilities.

## CUI Token Use-Cases

- **Transaction Fees**: For paying network and stability charges.
- **Governance Participation**: Vote on network upgrades.
- **Economic Utility**: Employed in system liquidations.
- **Discounts**: Reduced fees for DApps and wallets.
- **Staking**: Enhance network security.

## Cross-Chain Functionality

- Decentralized, trustless Ethereum bridge.
- Support for ERC-20, NFTs, and wrapped assets.
- Plugin-based customizable bridge features.

## Developer's Guide

### Setting Up

To prepare your environment, follow these instructions:

1. **Rust and Dependencies Installation**:

   - Install Rust and related tools:
     ```bash
     curl https://sh.rustup.rs -sSf | sh
     source $HOME/.cargo/env
     rustup component add rustfmt
     rustup update
     ```

   - For Linux (Ubuntu):
     ```bash
     sudo apt-get update
     sudo apt-get install libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang cmake make libprotobuf-dev protobuf-compiler
     ```

   - For Mac M1, install Rosetta:
     ```bash
     softwareupdate --install-rosetta
     ```

2. **Codebase Setup**:

   - Clone and navigate to the repo:
     ```bash
     git clone https://github.com/ChuiNetwork/chui.git
     cd chui
     ```

   - Build the code:
     ```bash
     cargo build
     ```

3. **Local Cluster Execution**:

   - Start local cluster:
     ```bash
     ./run.sh
     ```

### Testing and Performance

- **Testing**:

  - To run tests:
    ```bash
    cargo test --no-fail-fast
    ```

- **EVM Testing and Local Net**:

  - Consult [EVM Documentation](https://docs.chui.network/evm) for EVM integration and local testnet setup.

- **Benchmarking**:

  - Install Rust nightly build:
    ```bash
    rustup install nightly
    ```

  - Execute benchmarks:
    ```bash
    cargo +nightly bench
    ```

### Troubleshooting

If you encounter setup or development issues:

- Verify all dependencies are installed as per the setup guide.

- Update Rust:
  ```bash
  rustup update

Ensure build tools and libraries are properly installed for code building.

For specific issues, refer to the project's issue tracker or forums.

## Licensing

- **Release Information**: Detailed in [RELEASE.md](RELEASE.md).
- **Copyright**:

Copyright 2024 Chui Network
Licensed under Apache License, Version 2.0.
Available at http://www.apache.org/licenses/LICENSE-2.0


This version maintains a professional tone while streamlining the structure for clarity and conciseness.
