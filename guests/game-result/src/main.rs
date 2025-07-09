use std::io::Read;
use risc0_zkvm::guest::env;

fn main() {
    let mut data = Vec::<u8>::new();
    env::stdin().read_to_end(&mut data).unwrap();
    // 直接把输入写入journal，证明数据已被承诺
    env::commit_slice(&data);
} 