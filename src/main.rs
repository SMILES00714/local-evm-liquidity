mod provider;
mod vm;

use std::time::Instant;

use crate::{provider::Provider, vm::Vm};
use anyhow::Result;

const RUNS: usize = 100000;

fn main() -> Result<()> {
    let provider = Provider::from_env()?;
    let mut vm = Vm::new(provider)?;

    // CowProtocolToken.getBalance(GPv2Settlement)
    run(
        &mut vm,
        "0xDEf1CA1fb7FBcDC777520aa7f396b4E015F497aB",
        "0x70a082310000000000000000000000009008D19f58AAbD9eD0D60971565AA8510560ab41",
    )?;

    // UniswapV3Quoter.quoteExactSingleInput(WETH, COW, 3000, 1.0, 0)
    run(
        &mut vm,
        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6",
        "0xf7729d43\
           000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2\
           000000000000000000000000def1ca1fb7fbcdc777520aa7f396b4e015f497ab\
           0000000000000000000000000000000000000000000000000000000000000bb8\
           0000000000000000000000000000000000000000000000000de0b6b3a7640000\
           0000000000000000000000000000000000000000000000000000000000000000",
    )?;

    Ok(())
}

fn run(vm: &mut Vm, to: &str, data: &str) -> Result<()> {
    let mut call = vm.call_s(to, data)?;

    println!("priming...");
    let (output, gas) = call.execute()?;
    println!("output: 0x{}, gas: {}", hex::encode(output), gas);

    println!("starting {RUNS} runs...");
    let timer = Instant::now();
    for _ in 0..RUNS {
        call.execute()?;
    }
    println!("time: {:?}", timer.elapsed());

    Ok(())
}
