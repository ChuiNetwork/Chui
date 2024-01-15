use {
    solana_sdk::{pubkey::Pubkey, saturating_add_assign},
    std::collections::HashMap,
};

#[derive(Default, Debug, PartialEq)]
pub struct ProgramTiming {
    pub accumulated_us: u64,
    pub accumulated_units: u64,
    pub count: u32,
    pub errored_txs_compute_consumed: Vec<u64>,
    // Sum of all units in `errored_txs_compute_consumed`
    pub total_errored_units: u64,
}

impl ProgramTiming {
    pub fn coalesce_error_timings(&mut self, current_estimated_program_cost: u64) {
        for tx_error_compute_consumed in self.errored_txs_compute_consumed.drain(..) {
            let compute_units_update =
                std::cmp::max(current_estimated_program_cost, tx_error_compute_consumed);
            saturating_add_assign!(self.accumulated_units, compute_units_update);
            saturating_add_assign!(self.count, 1);
        }
    }

    pub fn accumulate_program_timings(&mut self, other: &ProgramTiming) {
        saturating_add_assign!(self.accumulated_us, other.accumulated_us);
        saturating_add_assign!(self.accumulated_units, other.accumulated_units);
        saturating_add_assign!(self.count, other.count);
        // Clones the entire vector, maybe not great...
        self.errored_txs_compute_consumed
            .extend(other.errored_txs_compute_consumed.clone());
        saturating_add_assign!(self.total_errored_units, other.total_errored_units);
    }
}

#[derive(Default, Debug)]
pub struct ExecuteTimings {
    pub check_us: u64,
    pub load_us: u64,
    pub execute_us: u64,
    pub store_us: u64,
    pub update_stakes_cache_us: u64,
    pub total_batches_len: usize,
    pub num_execute_batches: u64,
    pub collect_logs_us: u64,
    pub details: ExecuteDetailsTimings,
    pub execute_accessories: ExecuteAccessoryTimings,
}

impl ExecuteTimings {
    pub fn accumulate(&mut self, other: &ExecuteTimings) {
        saturating_add_assign!(self.check_us, other.check_us);
        saturating_add_assign!(self.load_us, other.load_us);
        saturating_add_assign!(self.execute_us, other.execute_us);
        saturating_add_assign!(self.store_us, other.store_us);
        saturating_add_assign!(self.update_stakes_cache_us, other.update_stakes_cache_us);
        saturating_add_assign!(self.total_batches_len, other.total_batches_len);
        saturating_add_assign!(self.num_execute_batches, other.num_execute_batches);
        saturating_add_assign!(self.collect_logs_us, other.collect_logs_us);
        self.details.accumulate(&other.details);
        self.execute_accessories
            .accumulate(&other.execute_accessories);
    }
}

#[derive(Default, Debug)]
pub struct ExecuteProcessInstructionTimings {
    pub total_us: u64,
    pub verify_caller_us: u64,
    pub process_executable_chain_us: u64,
    pub verify_callee_us: u64,
}

impl ExecuteProcessInstructionTimings {
    pub fn accumulate(&mut self, other: &ExecuteProcessInstructionTimings) {
        saturating_add_assign!(self.total_us, other.total_us);
        saturating_add_assign!(self.verify_caller_us, other.verify_caller_us);
        saturating_add_assign!(
            self.process_executable_chain_us,
            other.process_executable_chain_us
        );
        saturating_add_assign!(self.verify_callee_us, other.verify_callee_us);
    }
}

#[derive(Default, Debug)]
pub struct ExecuteAccessoryTimings {
    pub feature_set_clone_us: u64,
    pub compute_budget_process_transaction_us: u64,
    pub get_executors_us: u64,
    pub process_message_us: u64,
    pub update_executors_us: u64,
    pub process_instructions: ExecuteProcessInstructionTimings,
}

impl ExecuteAccessoryTimings {
    pub fn accumulate(&mut self, other: &ExecuteAccessoryTimings) {
        saturating_add_assign!(
            self.compute_budget_process_transaction_us,
            other.feature_set_clone_us
        );
        saturating_add_assign!(
            self.compute_budget_process_transaction_us,
            other.compute_budget_process_transaction_us
        );
        saturating_add_assign!(self.get_executors_us, other.get_executors_us);
        saturating_add_assign!(self.process_message_us, other.process_message_us);
        saturating_add_assign!(self.update_executors_us, other.update_executors_us);
        self.process_instructions
            .accumulate(&other.process_instructions);
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct ExecuteDetailsTimings {
    pub serialize_us: u64,
    pub create_vm_us: u64,
    pub create_evm_executor_us: u64,
    pub execute_us: u64,
    pub deserialize_us: u64,
    pub get_or_create_executor_us: u64,
    pub changed_account_count: u64,
    pub total_account_count: u64,
    pub total_data_size: usize,
    pub data_size_changed: usize,
    pub create_executor_register_syscalls_us: u64,
    pub create_executor_load_elf_us: u64,
    pub create_executor_verify_code_us: u64,
    pub create_executor_jit_compile_us: u64,
    pub per_program_timings: HashMap<Pubkey, ProgramTiming>,
}

impl ExecuteDetailsTimings {
    pub fn accumulate(&mut self, other: &ExecuteDetailsTimings) {
        saturating_add_assign!(self.serialize_us, other.serialize_us);
        saturating_add_assign!(self.create_vm_us, other.create_vm_us);
        saturating_add_assign!(self.create_evm_executor_us, other.create_evm_executor_us);
        saturating_add_assign!(self.execute_us, other.execute_us);
        saturating_add_assign!(self.deserialize_us, other.deserialize_us);
        saturating_add_assign!(
            self.get_or_create_executor_us,
            other.get_or_create_executor_us
        );
        saturating_add_assign!(self.changed_account_count, other.changed_account_count);
        saturating_add_assign!(self.total_account_count, other.total_account_count);
        saturating_add_assign!(self.total_data_size, other.total_data_size);
        saturating_add_assign!(self.data_size_changed, other.data_size_changed);
        saturating_add_assign!(
            self.create_executor_register_syscalls_us,
            other.create_executor_register_syscalls_us
        );
        saturating_add_assign!(
            self.create_executor_load_elf_us,
            other.create_executor_load_elf_us
        );
        saturating_add_assign!(
            self.create_executor_verify_code_us,
            other.create_executor_verify_code_us
        );
        saturating_add_assign!(
            self.create_executor_jit_compile_us,
            other.create_executor_jit_compile_us
        );
        for (id, other) in &other.per_program_timings {
            let program_timing = self.per_program_timings.entry(*id).or_default();
            program_timing.accumulate_program_timings(other);
        }
    }

    pub fn accumulate_program(
        &mut self,
        program_id: &Pubkey,
        us: u64,
        compute_units_consumed: u64,
        is_error: bool,
    ) {
        let program_timing = self.per_program_timings.entry(*program_id).or_default();
        program_timing.accumulated_us = program_timing.accumulated_us.saturating_add(us);
        if is_error {
            program_timing
                .errored_txs_compute_consumed
                .push(compute_units_consumed);
            program_timing.total_errored_units = program_timing
                .total_errored_units
                .saturating_add(compute_units_consumed);
        } else {
            program_timing.accumulated_units = program_timing
                .accumulated_units
                .saturating_add(compute_units_consumed);
            program_timing.count = program_timing.count.saturating_add(1);
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn construct_execute_timings_with_program(
        program_id: &Pubkey,
        us: u64,
        compute_units_consumed: u64,
    ) -> ExecuteDetailsTimings {
        let mut execute_details_timings = ExecuteDetailsTimings::default();

        // Accumulate an erroring transaction
        let is_error = true;
        execute_details_timings.accumulate_program(
            program_id,
            us,
            compute_units_consumed,
            is_error,
        );

        // Accumulate a non-erroring transaction
        let is_error = false;
        execute_details_timings.accumulate_program(
            program_id,
            us,
            compute_units_consumed,
            is_error,
        );

        let program_timings = execute_details_timings
            .per_program_timings
            .get(program_id)
            .unwrap();

        // Both error and success transactions count towards `accumulated_us`
        assert_eq!(program_timings.accumulated_us, us.saturating_mul(2));
        assert_eq!(program_timings.accumulated_units, compute_units_consumed);
        assert_eq!(program_timings.count, 1,);
        assert_eq!(
            program_timings.errored_txs_compute_consumed,
            vec![compute_units_consumed]
        );
        assert_eq!(program_timings.total_errored_units, compute_units_consumed,);

        execute_details_timings
    }

    #[test]
    fn test_execute_details_timing_acumulate_program() {
        // Acumulate an erroring transaction
        let program_id = Pubkey::new_unique();
        let us = 100;
        let compute_units_consumed = 1;
        construct_execute_timings_with_program(&program_id, us, compute_units_consumed);
    }

    #[test]
    fn test_execute_details_timing_acumulate() {
        // Acumulate an erroring transaction
        let program_id = Pubkey::new_unique();
        let us = 100;
        let compute_units_consumed = 1;
        let mut execute_details_timings = ExecuteDetailsTimings::default();

        // Construct another separate instance of ExecuteDetailsTimings with non default fields
        let mut other_execute_details_timings =
            construct_execute_timings_with_program(&program_id, us, compute_units_consumed);
        let account_count = 1;
        let data_size_changed = 1;
        other_execute_details_timings.serialize_us = us;
        other_execute_details_timings.create_vm_us = us;
        other_execute_details_timings.execute_us = us;
        other_execute_details_timings.deserialize_us = us;
        other_execute_details_timings.changed_account_count = account_count;
        other_execute_details_timings.total_account_count = account_count;
        other_execute_details_timings.total_data_size = data_size_changed;
        other_execute_details_timings.data_size_changed = data_size_changed;

        // Accumulate the other instance into the current instance
        execute_details_timings.accumulate(&other_execute_details_timings);

        // Check that the two instances are equal
        assert_eq!(execute_details_timings, other_execute_details_timings);
    }
}
