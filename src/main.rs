mod provider;
mod vm;

use crate::{provider::Provider, vm::Vm};
use anyhow::Result;
use std::time::Instant;

const RUNS: usize = 100000;

fn main() -> Result<()> {
    let provider = Provider::from_env()?;
    let mut vm = Vm::new(provider)?;

    run(
        "CowProtocolToken.getBalance(GPv2Settlement)",
        &mut vm,
        "0xDEf1CA1fb7FBcDC777520aa7f396b4E015F497aB",
        "0x70a08231\
           0000000000000000000000009008d19f58aabd9ed0d60971565aa8510560ab41",
    )?;

    // UniswapV3Quoter.quoteExactSingleInput(WETH, COW, 3000, 1.0, 0)
    run(
        "UniswapV3Quoter.quoteExactSingleInput(WETH, COW, 3000, 1.0, 0)",
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

fn run(name: &str, vm: &mut Vm, to: &str, data: &str) -> Result<()> {
    println!("# {name}");

    let mut call = vm.call_s(to, data)?;

    println!("priming...");
    // Make sure to execute at least once before timing. This is so our state
    // cache gets primed, and so we evaluate the performance of just the call
    // execution without data-fetching.
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
