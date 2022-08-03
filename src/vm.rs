use crate::provider::Provider;
use anyhow::{bail, ensure, Context as _, Result};
use primitive_types::{H160, H256, U256};
use revm::{
    db::{CacheDB, DatabaseRef},
    AccountInfo, Bytecode, Return, TransactOut, TransactTo, EVM, KECCAK_EMPTY,
};
use std::fmt::{self, Debug, Formatter};

#[derive(Debug)]
pub struct State {
    provider: Provider,
    block: U256,
}

impl State {
    pub fn new(provider: Provider) -> Result<Self> {
        let block = provider.block_number()? - 5;
        Ok(Self { provider, block })
    }
}

impl DatabaseRef for State {
    fn basic(&self, address: H160) -> AccountInfo {
        try_and_log_err(|| {
            let code = match self.provider.get_code(address, self.block)? {
                code if code.is_empty() => None,
                code => Some(Bytecode::new_raw(code.into())),
            };

            Ok(AccountInfo {
                balance: self.provider.get_balance(address, self.block)?,
                nonce: self
                    .provider
                    .get_transaction_count(address, self.block)?
                    .as_u64(),
                code_hash: code
                    .as_ref()
                    .map(|code| code.hash())
                    .unwrap_or(KECCAK_EMPTY),
                code,
            })
        })
    }

    fn code_by_hash(&self, _: H256) -> Bytecode {
        unimplemented!()
    }

    fn storage(&self, address: H160, index: U256) -> U256 {
        try_and_log_err(|| self.provider.get_storage_at(address, index, self.block))
    }

    fn block_hash(&self, _: U256) -> H256 {
        unimplemented!()
    }
}

fn try_and_log_err<T, F>(result: F) -> T
where
    T: Default,
    F: FnOnce() -> Result<T>,
{
    match result() {
        Ok(value) => value,
        Err(err) => {
            dbg!(err);
            T::default()
        }
    }
}

pub struct Vm {
    evm: EVM<CacheDB<State>>,
}

impl Vm {
    pub fn new(provider: Provider) -> Result<Self> {
        let db = CacheDB::new(State::new(provider)?);

        let mut evm = revm::new();
        evm.database(db);
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

impl Debug for Vm {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Vm")
            .field("env", &self.evm.env)
            .field("db", self.evm.db.as_ref().unwrap())
            .finish()
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
