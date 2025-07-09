#!/bin/bash

echo "🎲 启动猜数字游戏..."

# 检查环境变量
if [ -z "$PRIVATE_KEY" ]; then
    echo "❌ 错误: 请设置PRIVATE_KEY环境变量"
    echo "   export PRIVATE_KEY=\"your_private_key_here\""
    exit 1
fi

# 设置默认RPC_URL（如果没有设置）
if [ -z "$RPC_URL" ]; then
    export RPC_URL="https://ethereum-sepolia-rpc.publicnode.com"
    echo "🔗 使用默认RPC URL: $RPC_URL"
fi

# 启用开发模式（可选）
if [ "$1" = "--dev" ]; then
    export RISC0_DEV_MODE=1
    echo "🛠️  开发模式已启用（使用模拟证明）"
fi

echo "🔧 构建项目..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 构建失败"
    exit 1
fi

echo "🚀 启动服务器..."
echo "📱 游戏地址: http://localhost:3030"
echo "💡 请确保已安装MetaMask并连接到Sepolia测试网"
echo ""

cd apps && cargo run --release 