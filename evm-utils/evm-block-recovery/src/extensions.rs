use solana_evm_loader_program::instructions::v0;
use solana_sdk::{evm_loader::ID as STATIC_PROGRAM_ID, instruction::CompiledInstruction};
use solana_transaction_status::{
    ConfirmedBlockWithOptionalMetadata, TransactionWithOptionalMetadata,
};

#[derive(Debug)]
pub struct ParsedInstructions {
    pub instructions: Vec<v0::EvmInstruction>,
    pub only_trivial_instructions: bool,
    pub has_chui_account_instruction: bool,
}

impl ParsedInstructions {
    pub fn instr_evm_transaction(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| matches!(i, v0::EvmInstruction::EvmTransaction { .. }))
            .count()
    }

    pub fn instr_evm_swap_to_native(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| matches!(i, v0::EvmInstruction::SwapNativeToEther { .. }))
            .count()
    }

    pub fn instr_evm_free_ownership(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| matches!(i, v0::EvmInstruction::FreeOwnership {}))
            .count()
    }

    pub fn instr_evm_big_transaction(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| matches!(i, v0::EvmInstruction::EvmBigTransaction(_)))
            .count()
    }

    pub fn instr_evm_authorized_transaction(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| matches!(i, v0::EvmInstruction::EvmAuthorizedTransaction { .. }))
            .count()
    }
    pub fn can_produce_evm_block(&self) -> bool {
        self.instr_evm_transaction()
            + self.instr_evm_authorized_transaction()
            + self.instr_evm_swap_to_native()
            + self
                .instructions
                .iter()
                .filter(|i| {
                    matches!(
                        i,
                        v0::EvmInstruction::EvmBigTransaction(
                            v0::EvmBigTransaction::EvmTransactionExecute {}
                        ) | v0::EvmInstruction::EvmBigTransaction(
                            v0::EvmBigTransaction::EvmTransactionExecuteUnsigned { .. }
                        )
                    )
                })
                .count()
            > 0
            || self.has_chui_account_instruction
    }
}

pub trait NativeBlockExt {
    fn parse_instructions(&self) -> ParsedInstructions;
}

impl NativeBlockExt for ConfirmedBlockWithOptionalMetadata {
    fn parse_instructions(&self) -> ParsedInstructions {
        use std::str::FromStr;
        let chui_account_pk =
            solana_sdk::pubkey::Pubkey::from_str("VAcccHVjpknkW5N5R9sfRppQxYJrJYVV7QJGKchkQj5")
                .unwrap();
        let mut only_trivial_instructions = true;
        let mut instructions = vec![];
        let mut has_chui_account_instruction = false;

        for TransactionWithOptionalMetadata { transaction, .. } in &self.transactions {
            for CompiledInstruction {
                data,
                program_id_index,
                ..
            } in &transaction.message.instructions
            {
                if transaction.message.account_keys[*program_id_index as usize] == STATIC_PROGRAM_ID
                {
                    if data[0] == 0xff {
                        panic!("Borsh format is currently unsupported");
                    }
                    let instruction: v0::EvmInstruction = bincode::deserialize(data).unwrap();
                    match &instruction {
                        v0::EvmInstruction::EvmTransaction { .. } => (),
                        _ => only_trivial_instructions = false,
                    }

                    instructions.push(instruction);
                } else if transaction.message.account_keys[*program_id_index as usize]
                    == chui_account_pk
                    && transaction
                        .message
                        .account_keys
                        .iter()
                        .any(|k| *k == STATIC_PROGRAM_ID)
                {
                    has_chui_account_instruction = true;
                }
            }
        }

        ParsedInstructions {
            instructions,
            only_trivial_instructions,
            has_chui_account_instruction,
        }
    }
}
