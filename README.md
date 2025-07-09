# 🎲 基于Boundless的猜数字游戏

一个使用Boundless证明市场生成随机数的Web游戏。玩家连接MetaMask钱包，系统通过zkVM生成1-100的随机数，玩家猜测比实际数字高或低。

## ✨ 功能特点

- 🎯 **纯Web界面** - 无需钱包连接，直接开始游戏
- 🔐 **零知识证明** - 使用Boundless市场生成可验证的随机数
- 🎮 **实时游戏** - 动态状态更新和现代UI
- ⚡ **高性能** - Rust后端 + 原生JavaScript前端

## 🏗️ 技术架构

### zkVM Guest程序 (`guests/random-number/`)
```rust
// 使用线性同余生成器产生1-100的随机数
let seed: u64 = env::read();
let random = ((seed.wrapping_mul(1103515245).wrapping_add(12345)) % 100) + 1;
risc0_zkvm::guest::env::commit(&(random as u32));
```

### Web服务器 (`apps/src/main.rs`)
- 基于Warp框架的REST API
- Boundless客户端集成
- 游戏状态管理

### 前端 (`static/index.html`)
- 纯Web界面，无需钱包
- 实时状态轮询
- 现代渐变UI设计

## 🚀 快速开始

### 1. 环境准备

确保安装以下工具：
```bash
# 安装Rust和Cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装RISC Zero工具链
curl -L https://risczero.com/install | bash
rzup
```

### 2. 网络设置（服务器管理员配置）

#### Sepolia测试网配置
服务器管理员需要：
1. 准备一个钱包地址用于服务器
2. 获取测试ETH：https://faucets.chain.link/sepolia
3. 获取HitPoints测试代币（Boundless质押代币）

#### 重要合约地址 (Sepolia)
- **Boundless Market**: `0x13337C76fE2d1750246B68781ecEe164643b98Ec`
- **HitPoints代币**: `0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238`
- **Verifier Router**: `0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187`

### 3. 环境变量配置

复制环境变量模板：
```bash
cp example.env .env
```

编辑`.env`文件：
```bash
# 钱包私钥（不要泄露！）
PRIVATE_KEY=你的私钥

# Sepolia RPC端点
RPC_URL=https://ethereum-sepolia-rpc.publicnode.com

# 日志级别
RUST_LOG=info
```

### 4. 手动存款准备（服务器启动前）

⚠️ **重要：服务器管理员需要在启动游戏服务器前手动存款到Boundless市场**

#### 一次性存款操作

1. **ETH存款** - 支付证明费用
   ```bash
   # 每次证明大约需要 0.001-0.002 ETH
   # 建议存款足够多次游戏使用
   # 需要直接调用Boundless Market合约的 deposit() 方法
   ```

2. **HitPoints代币存款** - 支付lock stake质押  
   ```bash
   # 每次请求需要质押约 0.001 HitPoints
   # 建议存款足够多次游戏使用
   # 先调用 HitPoints.approve(market_address, amount)
   # 再调用 BoundlessMarket.depositStake(amount)
   ```

#### 管理员存款步骤

**通过MetaMask或其他钱包界面：**

1. **存入ETH**：
   - 合约地址：`0x13337C76fE2d1750246B68781ecEe164643b98Ec`
   - 调用方法：`deposit()` 
   - 发送ETH：0.1 ETH (建议存足够多次游戏)

2. **存入HitPoints质押**：
   - 先授权：向 `0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238` 调用 `approve(0x13337C76fE2d1750246B68781ecEe164643b98Ec, 10000000)`
   - 再存入：向 `0x13337C76fE2d1750246B68781ecEe164643b98Ec` 调用 `depositStake(10000000)`

**获取HitPoints测试代币：**
- 如果你有ETH，可以通过Uniswap等DEX交换
- 或联系Boundless社区获取测试代币

### 5. 构建和运行

```bash
# 构建项目
cargo build --release

# 运行服务器
./run.sh
# 或者
cargo run --bin apps
```

访问 http://localhost:3030 开始游戏！

## 🎮 游戏流程

1. **打开游戏** - 访问游戏网址
2. **开始游戏** - 点击"开始新游戏"按钮
3. **等待证明** - zkVM生成随机数证明（约30秒-2分钟）
4. **开始猜测** - 选择"更高"、"更低"或"相等"
5. **查看结果** - 显示实际数字和游戏结果

## 💰 费用说明

### 服务器运营成本
服务器管理员需要提前存款到Boundless市场：

1. **ETH存款** - 支付证明费用
   - 每次证明费用：~0.001-0.002 ETH
   - 建议存款：0.1 ETH (可支持50+次游戏)
   - 存款方法：调用市场合约的 `deposit()` 方法

2. **HitPoints质押** - 支付锁定质押  
   - 每次质押金额：~0.001 HitPoints
   - 建议存款：10,000,000 wei (可支持多次游戏)
   - 完成后会退还质押
   - 存款方法：先 `approve()` 再 `depositStake()`

### 用户使用成本
- **免费游戏**: 用户无需支付任何费用
- **测试网**: 服务器使用测试代币，无实际成本
- **主网**: 服务器运营成本约每次游戏 0.002 ETH + gas费用

## 🔧 开发模式

如果没有足够的测试代币，可以启用开发模式：

```bash
# 在apps/src/main.rs中取消注释开发模式代码
// 使用本地随机数而不是Boundless证明
```

## 📊 系统架构图

```
预先准备：服务器配置私钥和手动存款ETH和HitPoints到Boundless市场

用户 → Web界面 → 后端服务器 → Boundless市场 → 证明者网络
 ↓                                ↓              ↓
开始游戏          创建证明请求      使用已存款      生成证明
 ↓                                ↓              ↓
等待证明          提交请求         扣除费用        返回结果
```

## 🐛 常见问题

### Q: "需要设置PRIVATE_KEY环境变量"
A: 确保.env文件中设置了正确的私钥（服务器管理员负责配置）

### Q: "Insufficient balance to cover request"  
A: 服务器管理员需要手动存入更多ETH到Boundless市场，调用市场合约的 `deposit()` 方法

### Q: 提交请求时出现质押相关错误
A: 服务器管理员需要确保已手动存入足够的HitPoints质押，需要先 `approve()` 再 `depositStake()`

### Q: 如何检查Boundless市场中的余额？
A: 服务器管理员可以调用市场合约的 `balanceOf(服务器地址)` 和 `balanceOfStake(服务器地址)` 方法

### Q: 证明生成时间很长
A: Boundless网络负载可能较高，通常1-5分钟内完成

### Q: 游戏状态一直显示"requesting_proof"
A: 检查网络连接和Boundless服务状态，或检查是否有足够的存款余额（服务器管理员负责）

## 📝 相关资源

- [Boundless文档](https://docs.beboundless.xyz)
- [RISC Zero文档](https://dev.risczero.com)
- [Sepolia测试网水龙头](https://faucets.chain.link/sepolia)
- [MetaMask设置指南](https://metamask.io/download/)

## 📝 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件 