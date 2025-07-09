#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use risc0_zkvm::guest::env;
use core::convert::TryInto;
risc0_zkvm::guest::entry!(main);

fn main() {
    // 按默认 codec 顺序读取 random_number, guess, won
    let random: u32 = env::read();
    let guess: u32 = env::read();
    let won: u8 = env::read();

    let mut output = Vec::new();
    output.extend(&random.to_le_bytes());
    output.extend(&guess.to_le_bytes());
    output.push(won);

    env::commit_slice(&output);
} 