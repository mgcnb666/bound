# 🎲 Boundless 猜数字游戏完整部署指南

> 使用 RISC Zero zkVM + Boundless 市场生成可验证随机数，配合 Warp 后端与纯静态前端实现的猜数字小游戏。

---

## 目录
1. [功能简介](#功能简介)
2. [整体架构](#整体架构)
3. [先决条件](#先决条件)
4. [环境准备](#环境准备)
5. [编译与运行](#编译与运行)
6. [域名 & HTTPS 部署](#域名--https-部署)
7. [常见问题](#常见问题)
8. [参考链接](#参考链接)

---

## 功能简介
* 服务器本地生成 **1-100** 的随机数 → 通过 **game-result** guest 在 zkVM 中写入 journal。
* 后端使用 **Boundless CLI** 将 ELF + 输入上传（Pinata / S3 / 本地）并提交请求。
* 用户只需浏览器即可游玩（无钱包依赖）。

---

## 整体架构
```
┌────────┐    HTTP/JSON     ┌─────────────┐
│ Browser│ ───────────────► │ Warp Server │
└────────┘  3030            │  apps/      │
                             │            │   spawn_cli_proof()
                             └────┬───────┘
                                  │ boundless request submit-offer
                                  ▼
                           ┌──────────────┐
                           │ Boundless CLI│ (标准客户端)
                           └────┬─────────┘
            上传 ELF+输入          │
            (Pinata/S3/文件)      ▼
                           ┌──────────────┐
                           │ Boundless 网 │ (Prover 网络)
                           └──────────────┘
```

---

## 先决条件
| 组件 | 版本 | 说明 |
|------|------|------|
| Rust & Cargo | 1.76+ | 主机 & guest 编译 |
| RISC Zero 工具链 | 2.2 | `rzup install cargo-risczero r0vm` |
| Docker (+ buildx) | 24+ | 用于交叉编译 guest ELF |
| Boundless CLI | 0.3.0+ | `cargo install boundless-cli --locked` 或下载官方二进制 |
| Node.js (可选) | 18+ | 仅前端开发需要 |
| Nginx & Certbot (生产) | 最新 | 反向代理 + TLS |

> **测试网依赖**：一个 Sepolia 钱包私钥 & 少量 ETH + HitPoints(HP) 代币用于支付费用。

---

## 环境准备
```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 2. 安装 RISC Zero 工具链
curl -L https://github.com/risc0/risc0/releases/download/v2.2.0/installer.sh | bash
rzup install cargo-risczero r0vm

# 3. 安装 Docker (如未安装)
# 参见 https://docs.docker.com/engine/install/ubuntu/

# 4. 安装 Boundless CLI
cargo install boundless-cli --locked

# 5. Clone 项目
git clone https://github.com/your_org/guess-number-game.git
cd guess-number-game
```

### 手动安装步骤（Ubuntu 22.04）

以下步骤逐条执行，完成所有依赖安装，可根据自身环境跳过已满足的部分。

#### 1. 安装系统依赖
```bash
sudo apt update
sudo apt install -y curl wget git build-essential make gcc \
    pkg-config libssl-dev clang ninja-build lz4 jq tmux htop ncdu unzip \
    libgbm1 libclang-dev libleveldb-dev automake autoconf bsdmainutils \
    iptables nvme-cli tar ca-certificates gnupg
```

#### 2. 安装 Docker + buildx
```bash
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo tee /etc/apt/keyrings/docker.asc >/dev/null
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin
sudo systemctl enable --now docker
```

#### 3. 安装 Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update
```

#### 4. 安装 RISC Zero 工具链
```bash
curl -L https://risczero.com/install | bash
source ~/.bashrc
rzup install rust
rzup install cargo-risczero
```

#### 5. （可选）安装 cargo-risczero
```bash
cargo install cargo-risczero --locked
```

#### 6. 安装 Boundless CLI & Bento CLI
```bash
cargo install --locked boundless-cli
cargo install --locked --git https://github.com/risc0/risc0 bento-client --branch release-2.1 --bin bento_cli
```

#### 7. （可选）安装 just
```bash
cargo install just
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### 8. 版本检查
```bash
rustc --version && cargo --version
rzup --version && r0vm --version
docker --version && docker buildx version
boundless --help
```

创建 `.env`：
```bash
cat > .env <<EOF
RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
PRIVATE_KEY=0xYourPrivateKey
PINATA_JWT=eyJhbGciOiJ...
RUST_LOG=info
EOF
```

---

## 编译与运行
### 1. 编译 guest (game-result)
```bash
# 生成 riscv ELF；Docker 自动触发 buildx
cargo risczero build -p game-result
# 生成文件: target/riscv32im-risc0-zkvm-elf/docker/game-result.bin
```

### 2. 编译 & 启动后端
```bash
cargo build -p apps --release
# 运行 (使用 .env 环境变量)
source .env
./target/release/guess-number-app
# 默认监听 0.0.0.0:3030
```

### 3. 访问
浏览器打开 `http://<服务器IP>:3030/` 即可开始游戏。

---


---

## 常见问题
| 现象 | 可能原因 / 解决办法 |
|------|--------------------|
| guest 构建报 `DeserializeUnexpectedEnd` | host/guest 编码方式不一致；确保 `--encode-input` 与 guest `env::read()` 搭配 |
| `r0vm server incompatible` | 升级/降级 `r0vm` 与 `risc0-zkvm` 版本一致（均 2.2） |
| Boundless CLI 报 *storage provider required* | 设置 `PINATA_JWT` 或改用 `RISC0_DEV_MODE=1` 本地文件存储 |
| 请求价格过高 | 在 `apps/src/main.rs` 的 `spawn_cli_proof` 里通过 `--min-price / --max-price` 覆盖 |
| 反向代理后 WebSocket 404 | Warp 无 WebSocket，本项目纯 HTTP；若自定义添加需在 Nginx 加 `proxy_http_version 1.1;` |

---

## 参考链接
* Boundless 文档：https://docs.beboundless.xyz
* RISC Zero 开发文档：https://dev.risczero.com
* Let’s Encrypt Certbot：https://certbot.eff.org

---

> 本项目基于 MIT 许可证发布。 