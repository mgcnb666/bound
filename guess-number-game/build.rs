use std::path::Path;

fn main() {
    // 生成random-number guest的ELF文件和image ID
    risc0_build::embed_methods_with_options(
        risc0_build::DockerOptions::default(),
        risc0_build::GuestOptions::default(),
    );
} 