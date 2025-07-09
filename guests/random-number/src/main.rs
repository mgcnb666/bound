#![no_std]
#![no_main]

extern crate alloc;
use risc0_zkvm::guest::env;
use risc0_zkvm::guest::entry;

entry!(main);

fn main() {
    // 读取一个 u64 作为种子
    let seed: u64 = env::read();

    let random_value = generate_random_number(seed as u128, 1, 100);

    // 输出随机数到 journal
    env::commit_slice(&random_value.to_le_bytes());
}

/// 使用线性同余生成器生成 [min,max] 随机数
fn generate_random_number(mut seed: u128, min: u32, max: u32) -> u32 {
    const A: u128 = 1664525;
    const C: u128 = 1013904223;
    const M: u128 = 1u128 << 32;

    seed = (A * seed + C) % M;

    let range = (max - min + 1) as u128;
    (seed % range + min as u128) as u32
} 