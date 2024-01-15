use num_derive::{FromPrimitive, ToPrimitive};
use solana_sdk::decode_error::DecodeError;

use snafu::Snafu;

/// Reasons the evm execution can fail.
#[derive(Debug, Clone, PartialEq, FromPrimitive, ToPrimitive, Snafu)]
#[snafu(context(suffix(false)))]
pub enum EvmError {
    #[snafu(display("Cross-Program EVM execution disabled."))]
    CrossExecutionNotEnabled,

    #[snafu(display("InvokeContext didn't provide evm executor."))]
    EvmExecutorNotFound,

    #[snafu(display("Recursive cross-program EVM execution disabled."))]
    RecursiveCrossExecution,

    #[snafu(display("Internal executor error."))]
    InternalExecutorError,

    #[snafu(display("Internal transaction error."))]
    InternalTransactionError,

    #[snafu(display("Instruction expect additional account as argument."))]
    MissingAccount,

    #[snafu(display("Instruction expect some account to be a signer."))]
    MissingRequiredSignature,

    #[snafu(display("Authorized transaction EVM address should be calculated from sender address using evm_address_for_program."))]
    AuthorizedTransactionIncorrectAddress,

    #[snafu(display("Wrong AuthorizedTx account owner.."))]
    AuthorizedTransactionIncorrectOwner,

    #[snafu(display("Cannot free ownership of an account that EVM didn't own."))]
    FreeNotEvmAccount,

    #[snafu(display("Cannot process swap, sender has not enough tokens."))]
    SwapInsufficient,

    #[snafu(display("Internal Error: Cannot borrow some of account."))]
    BorrowingFailed,

    #[snafu(display("Failed to allocate space in storage account."))]
    AllocateStorageFailed,

    #[snafu(display("Failed to write data into storage account."))]
    WriteStorageFailed,

    #[snafu(display("Failed to deserialize data from account."))]
    DeserializationError,

    #[snafu(display("EVM Transaction was reverted."))]
    RevertTransaction,

    #[snafu(display("This instruction is not supported yet."))]
    InstructionNotSupportedYet,

    #[snafu(display("This instruction cause overflow in fee refund calculation."))]
    OverflowInRefund,

    #[snafu(display("Native account has not enough tokens."))]
    NativeAccountInsufficientFunds, // TODO: can we merge this with SwapInsufficient?

    #[snafu(display("Precompile error"))]
    PrecompileError,
}

impl<E> DecodeError<E> for EvmError {
    fn type_of() -> &'static str {
        "EvmError"
    }
}
