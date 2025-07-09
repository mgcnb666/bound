use std::io::Read;
use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use risc0_zkvm::guest::env;
use risc0_zkvm::sha::Digest;

fn main() {
    // 从主机读取种子值（时间戳 + 区块哈希等）
    let mut seed_bytes = Vec::<u8>::new();
    env::stdin().read_to_end(&mut seed_bytes).unwrap();
    
    // 解码种子
    let seed = <U256>::abi_decode(&seed_bytes).unwrap();
    
    // 使用种子生成伪随机数
    // 我们使用简单的线性同余生成器来生成随机数
    let random_value = generate_random_number(seed, 1, 100);
    
    // 将随机数提交到journal
    env::commit_slice(&random_value.abi_encode());
}

/// 使用线性同余生成器生成指定范围内的随机数
fn generate_random_number(seed: U256, min: u32, max: u32) -> U256 {
    // LCG参数 (来自Numerical Recipes)
    let a = U256::from(1664525u32);
    let c = U256::from(1013904223u32);
    let m = U256::from(2u32).pow(U256::from(32u32)); // 2^32
    
    // 应用LCG公式: (a * seed + c) % m
    let random = (a * seed + c) % m;
    
    // 将结果映射到指定范围 [min, max]
    let range = max - min + 1;
    let result = (random % U256::from(range)) + U256::from(min);
    
    result
} 