use {
    assert_matches::assert_matches,
    rand::thread_rng,
    solana_program_test::*,
    solana_sdk::{
        ed25519_instruction::new_ed25519_instruction,
        feature_set,
        signature::Signer,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
};

#[tokio::test]
async fn test_success() {
    let mut context = ProgramTest::default().start_with_context().await;

    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;

    let privkey = ed25519_dalek::Keypair::generate(&mut thread_rng());
    let message_arr = b"hello";
    let instruction = new_ed25519_instruction(&privkey, message_arr);

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    assert_matches!(client.process_transaction(transaction).await, Ok(()));
}

#[tokio::test]
async fn test_failure() {
    let mut context = ProgramTest::default().start_with_context().await;

    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;

    let privkey = ed25519_dalek::Keypair::generate(&mut thread_rng());
    let message_arr = b"hello";
    let mut instruction = new_ed25519_instruction(&privkey, message_arr);

    instruction.data[0] += 1;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    assert_matches!(
        client.process_transaction(transaction).await,
        Err(TransportError::TransactionError(
            TransactionError::InvalidAccountIndex
        ))
    );
}

#[tokio::test]
async fn test_success_call_builtin_program() {
    let mut program_test = ProgramTest::default();
    program_test.deactivate_feature(feature_set::prevent_calling_precompiles_as_programs::id());
    let mut context = program_test.start_with_context().await;

    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;

    let privkey = ed25519_dalek::Keypair::generate(&mut thread_rng());
    let message_arr = b"hello";
    let instruction = new_ed25519_instruction(&privkey, message_arr);

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    assert_matches!(client.process_transaction(transaction).await, Ok(()));
}
