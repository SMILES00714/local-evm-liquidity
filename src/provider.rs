use anyhow::{Context as _, Result};
use curl::easy::{Easy, List};
use primitive_types::{H160, U256};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{env, io::Read as _};

#[derive(Debug)]
pub struct Provider {
    url: String,
}

#[derive(Deserialize)]
struct Response<T> {
    result: T,
}

impl Provider {
    pub fn from_env() -> Result<Self> {
        let url = env::var("NODE_URL")
            .or_else(|_| {
                env::var("INFURA_PROJECT_ID").map(|id| format!("https://mainnet.infura.io/v3/{id}"))
            })
            .context("missing NODE_URL or INFURA_PROJECT_ID environment variable")?;
        Ok(Self { url })
    }

    fn exec<T, U>(&self, method: &str, params: T) -> Result<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let result = self.try_exec(method, params);
        if let Err(err) = &result {
            dbg!(err);
        }
        result
    }

    fn try_exec<T, U>(&self, method: &str, params: T) -> Result<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let request = serde_json::to_vec(&json!({
            "id": 42,
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))?;
        let mut request = &request[..];
        let mut response = Vec::new();

        let mut easy = Easy::new();
        easy.url(&self.url)?;
        easy.post(true)?;
        easy.post_field_size(request.len().try_into()?)?;

        let mut list = List::new();
        list.append("Content-Type: application/json")?;
        easy.http_headers(list)?;

        {
            let mut transfer = easy.transfer();
            transfer.read_function(|buf| Ok(request.read(buf).unwrap_or(0)))?;
            transfer.write_function(|data| {
                response.extend_from_slice(data);
                Ok(data.len())
            })?;
            transfer.perform()?;
        }

        Ok(serde_json::from_slice::<Response<_>>(&response)?.result)
    }

    pub fn block_number(&self) -> Result<U256> {
        self.exec("eth_blockNumber", [(); 0])
    }

    pub fn get_balance(&self, account: H160, block: U256) -> Result<U256> {
        self.exec("eth_getBalance", (account, block))
    }

    pub fn get_code(&self, account: H160, block: U256) -> Result<Vec<u8>> {
        let hex = self.exec::<_, String>("eth_getCode", (account, block))?;
        let code = hex::decode(hex.strip_prefix("0x").context("missing 0x- prefix")?)?;
        Ok(code)
    }

    pub fn get_transaction_count(&self, account: H160, block: U256) -> Result<U256> {
        self.exec("eth_getTransactionCount", (account, block))
    }

    pub fn get_storage_at(&self, account: H160, position: U256, block: U256) -> Result<U256> {
        self.exec("eth_getStorageAt", (account, position, block))
    }
}
