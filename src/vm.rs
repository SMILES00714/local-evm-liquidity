use crate::provider::Provider;
use anyhow::{bail, ensure, Context as _, Result};
use primitive_types::{H160, H256, U256};
use revm::{AccountInfo, Bytecode, Database, Return, TransactOut, TransactTo, EVM, KECCAK_EMPTY};
use std::collections::HashMap;

#[derive(Debug)]
pub struct State {
    provider: Provider,
    cache: Cache,
}

#[derive(Debug, Default)]
struct Cache {
    block: U256,
    accounts: HashMap<H160, AccountInfo>,
    storage: HashMap<(H160, U256), U256>,
}

impl State {
    pub fn new(provider: Provider) -> Result<Self> {
        let block = provider.block_number()? - 5;
        Ok(Self {
            provider,
            cache: Cache {
                block,
                ..Default::default()
            },
        })
    }
}

impl Database for State {
    fn basic(&mut self, address: H160) -> AccountInfo {
        self.cache
            .accounts
            .entry(address)
            .or_insert_with(|| {
                load_account_info(&self.provider, address, self.cache.block).unwrap_or_default()
            })
            .clone()
    }

    fn code_by_hash(&mut self, _: H256) -> Bytecode {
        unimplemented!()
    }

    fn storage(&mut self, address: H160, index: U256) -> U256 {
        *self
            .cache
            .storage
            .entry((address, index))
            .or_insert_with(|| {
                self.provider
                    .get_storage_at(address, index, self.cache.block)
                    .unwrap_or_default()
            })
    }

    fn block_hash(&mut self, _: U256) -> H256 {
        unimplemented!()
    }
}

fn load_account_info(provider: &Provider, address: H160, block: U256) -> Result<AccountInfo> {
    let code = match provider.get_code(address, block)? {
        code if code.is_empty() => None,
        code => Some(Bytecode::new_raw(code.into())),
    };

    Ok(AccountInfo {
        balance: provider.get_balance(address, block)?,
        nonce: provider.get_transaction_count(address, block)?.as_u64(),
        code_hash: code
            .as_ref()
            .map(|code| code.hash())
            .unwrap_or(KECCAK_EMPTY),
        code,
    })
}

pub struct Vm {
    evm: EVM<State>,
}

impl Vm {
    pub fn new(provider: Provider) -> Result<Self> {
        let mut evm = revm::new();
        evm.database(State::new(provider)?);
        evm.env.cfg.perf_all_precompiles_have_balance = true;

        Ok(Self { evm })
    }

    pub fn call(&mut self, to: H160, data: Vec<u8>) -> Call<'_> {
        self.evm.env.tx.transact_to = TransactTo::Call(to);
        self.evm.env.tx.data = data.into();
        Call(self)
    }

    pub fn call_s(&mut self, to: &str, data: &str) -> Result<Call<'_>> {
        Ok(self.call(
            to.parse()?,
            hex::decode(data.strip_prefix("0x").context("missing 0x- prefix")?)?,
        ))
    }
}

pub struct Call<'vm>(&'vm mut Vm);

impl Call<'_> {
    pub fn execute(&mut self) -> Result<(Vec<u8>, U256)> {
        let (ret, out, gas, ..) = self.0.evm.transact();

        ensure!(matches!(ret, Return::Return), "call execution failed");
        let out = match out {
            TransactOut::Call(out) => out,
            _ => bail!("unexpected transaction output"),
        };

        Ok((out.to_vec(), gas.into()))
    }
}
