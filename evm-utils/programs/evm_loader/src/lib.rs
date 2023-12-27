extern crate core;

mod account_structure;
pub mod tx_chunks;

pub mod error;
pub mod instructions;
pub mod precompiles;
pub mod processor;
pub mod chui_extension;

pub static ID: chui_sdk::pubkey::Pubkey = chui_sdk::evm_loader::ID;

pub use account_structure::AccountStructure;
pub use processor::EvmProcessor;

/// Public API for intermediate eth <-> chuitransfers
pub mod scope {
    pub mod evm {
        pub use evm_state::transactions::*;
        pub use evm_state::*;
        pub use primitive_types::H160 as Address;

        pub const LAMPORTS_TO_GWEI_PRICE: u64 = 1_000_000_000; // Lamports is 1/10^9 of CUIs while GWEI is 1/10^18

        // Convert lamports to gwei
        pub fn lamports_to_gwei(lamports: u64) -> U256 {
            U256::from(lamports) * U256::from(LAMPORTS_TO_GWEI_PRICE)
        }

        // Convert gweis back to lamports, return change as second element.
        pub fn gweis_to_lamports(gweis: U256) -> (u64, U256) {
            let lamports = gweis / U256::from(LAMPORTS_TO_GWEI_PRICE);
            let gweis = gweis % U256::from(LAMPORTS_TO_GWEI_PRICE);
            (lamports.as_u64(), gweis)
        }
    }
    pub mod chui{
        pub use chui_sdk::{
            evm_state, instruction::Instruction, pubkey::Pubkey as Address,
            transaction::Transaction,
        };
    }
}
use instructions::{
    v0, EvmBigTransaction, EvmInstruction, ExecuteTransaction, FeePayerType,
    EVM_INSTRUCTION_BORSH_PREFIX,
};
use scope::*;
use chui_sdk::instruction::{AccountMeta, Instruction};

/// Create an evm instruction and add EVM_INSTRUCTION_BORSH_PREFIX prefix
/// at the beginning of instruction data to mark Borsh encoding
pub fn create_evm_instruction_with_borsh(
    program_id: chui_sdk::pubkey::Pubkey,
    data: &EvmInstruction,
    accounts: Vec<AccountMeta>,
) -> chui::Instruction {
    let mut res = Instruction::new_with_borsh(program_id, data, accounts);
    res.data.insert(0, EVM_INSTRUCTION_BORSH_PREFIX);
    res
}

/// Create an old version of evm instruction
pub fn create_evm_instruction_with_bincode(
    program_id: chui_sdk::pubkey::Pubkey,
    data: &v0::EvmInstruction,
    accounts: Vec<AccountMeta>,
) -> chui::Instruction {
    Instruction::new_with_bincode(program_id, data, accounts)
}

pub fn send_raw_tx(
    signer: chui::Address,
    evm_tx: evm::Transaction,
    gas_collector: Option<chui::Address>,
    fee_type: FeePayerType,
) -> chui::Instruction {
    let mut account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(signer, true),
    ];
    if let Some(gas_collector) = gas_collector {
        account_metas.push(AccountMeta::new(gas_collector, false))
    }

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::ExecuteTransaction {
            tx: ExecuteTransaction::Signed { tx: Some(evm_tx) },
            fee_type,
        },
        account_metas,
    )
}

pub fn authorized_tx(
    sender: chui::Address,
    unsigned_tx: evm::UnsignedTransaction,
    fee_type: FeePayerType,
) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(sender, true),
    ];

    let from = evm_address_for_program(sender);
    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::ExecuteTransaction {
            tx: ExecuteTransaction::ProgramAuthorized {
                tx: Some(unsigned_tx),
                from,
            },
            fee_type,
        },
        account_metas,
    )
}

pub(crate) fn transfer_native_to_evm(
    owner: chui::Address,
    lamports: u64,
    evm_address: evm::Address,
) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(owner, true),
    ];

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::SwapNativeToEther {
            lamports,
            evm_address,
        },
        account_metas,
    )
}

pub fn free_ownership(owner: chui::Address) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(owner, true),
    ];

    create_evm_instruction_with_borsh(crate::ID, &EvmInstruction::FreeOwnership {}, account_metas)
}

pub fn big_tx_allocate(storage: chui::Address, size: usize) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    let big_tx = EvmBigTransaction::EvmTransactionAllocate { size: size as u64 };

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::EvmBigTransaction(big_tx),
        account_metas,
    )
}

pub fn big_tx_write(storage: chui::Address, offset: u64, chunk: Vec<u8>) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    let big_tx = EvmBigTransaction::EvmTransactionWrite {
        offset,
        data: chunk,
    };

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::EvmBigTransaction(big_tx),
        account_metas,
    )
}

pub fn big_tx_execute(
    storage: chui::Address,
    gas_collector: Option<&chui::Address>,
    fee_type: FeePayerType,
) -> chui::Instruction {
    let mut account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    if let Some(gas_collector) = gas_collector {
        account_metas.push(AccountMeta::new(*gas_collector, false))
    }

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::ExecuteTransaction {
            tx: ExecuteTransaction::Signed { tx: None },
            fee_type,
        },
        account_metas,
    )
}
pub fn big_tx_execute_authorized(
    storage: chui::Address,
    from: evm::Address,
    gas_collector: chui::Address,
    fee_type: FeePayerType,
) -> chui::Instruction {
    let mut account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    if gas_collector != storage {
        account_metas.push(AccountMeta::new_readonly(gas_collector, true))
    }

    create_evm_instruction_with_borsh(
        crate::ID,
        &EvmInstruction::ExecuteTransaction {
            tx: ExecuteTransaction::ProgramAuthorized { tx: None, from },
            fee_type,
        },
        account_metas,
    )
}

pub fn transfer_native_to_evm_ixs(
    owner: chui::Address,
    lamports: u64,
    ether_address: evm::Address,
) -> Vec<chui::Instruction> {
    vec![
        chui_sdk::system_instruction::assign(&owner, &crate::ID),
        transfer_native_to_evm(owner, lamports, ether_address),
        free_ownership(owner),
    ]
}

/// Create an account that represent evm locked lamports count.
pub fn create_state_account(lamports: u64) -> chui_sdk::account::AccountSharedData {
    chui_sdk::account::Account {
        lamports: lamports + 1,
        owner: crate::ID,
        data: b"Evm state".to_vec(),
        executable: false,
        rent_epoch: 0,
    }
    .into()
}

///
/// Calculate evm::Address for chui::Pubkey, that can be used to call transaction from chui::bpf scope, into evm scope.
/// Native chain address is hashed and prefixed with [0xac, 0xc0] bytes.
///
pub fn evm_address_for_program(program_account: chui::Address) -> evm::Address {
    use primitive_types::{H160, H256};
    use sha3::{Digest, Keccak256};

    const ADDR_PREFIX: &[u8] = &[0xAC, 0xC0]; // ACC prefix for each account

    let addr_hash = Keccak256::digest(&program_account.to_bytes());
    let hash_bytes = H256::from_slice(addr_hash.as_slice());
    let mut short_hash = H160::from(hash_bytes);
    short_hash.as_bytes_mut()[0..2].copy_from_slice(ADDR_PREFIX);

    short_hash
}

pub fn evm_transfer(
    from: evm::SecretKey,
    to: evm::Address,
    nonce: evm::U256,
    value: evm::U256,
    chain_id: Option<u64>,
) -> evm::Transaction {
    let tx = evm::UnsignedTransaction {
        nonce,
        gas_price: 1.into(),
        gas_limit: 21000.into(),
        action: evm::TransactionAction::Call(to),
        value,
        input: vec![],
    };
    tx.sign(&from, chain_id)
}

// old instructions for emv bridge

pub fn send_raw_tx_old(
    signer: chui::Address,
    evm_tx: evm::Transaction,
    gas_collector: Option<chui::Address>,
) -> chui::Instruction {
    let mut account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(signer, true),
    ];
    if let Some(gas_collector) = gas_collector {
        account_metas.push(AccountMeta::new(gas_collector, false))
    }

    create_evm_instruction_with_bincode(
        crate::ID,
        &v0::EvmInstruction::EvmTransaction { evm_tx },
        account_metas,
    )
}

pub fn big_tx_allocate_old(storage: chui::Address, size: usize) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    let big_tx = v0::EvmBigTransaction::EvmTransactionAllocate { size: size as u64 };

    create_evm_instruction_with_bincode(
        crate::ID,
        &v0::EvmInstruction::EvmBigTransaction(big_tx),
        account_metas,
    )
}

pub fn big_tx_write_old(
    storage: chui::Address,
    offset: u64,
    chunk: Vec<u8>,
) -> chui::Instruction {
    let account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    let big_tx = v0::EvmBigTransaction::EvmTransactionWrite {
        offset,
        data: chunk,
    };

    create_evm_instruction_with_bincode(
        crate::ID,
        &v0::EvmInstruction::EvmBigTransaction(big_tx),
        account_metas,
    )
}

pub fn big_tx_execute_old(
    storage: chui::Address,
    gas_collector: Option<&chui::Address>,
) -> chui::Instruction {
    let mut account_metas = vec![
        AccountMeta::new(chui::evm_state::ID, false),
        AccountMeta::new(storage, true),
    ];

    if let Some(gas_collector) = gas_collector {
        account_metas.push(AccountMeta::new(*gas_collector, false))
    }

    let big_tx = v0::EvmBigTransaction::EvmTransactionExecute {};

    create_evm_instruction_with_bincode(
        crate::ID,
        &v0::EvmInstruction::EvmBigTransaction(big_tx),
        account_metas,
    )
}
