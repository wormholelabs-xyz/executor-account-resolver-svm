use anchor_lang::prelude::*;
use executor_account_resolver_svm::{InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1};

declare_id!("GeSLWQHGZRWhrdqo5Zvaa3JonhzQmfEmJSuHJwmRebPw");

#[program]
pub mod executor_account_resolver_svm_program {
    use super::*;

    #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
    pub fn resolve_execute_vaa_v1(
        _ctx: Context<Resolve>,
        _vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        Ok(Resolver::Resolved(InstructionGroups(vec![])))
    }
}

#[derive(Accounts)]
pub struct Resolve {}
