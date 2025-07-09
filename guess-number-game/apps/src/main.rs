use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH}
};

use alloy::{
    primitives::{Address, B256, U256, utils::parse_ether},
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use boundless_market::{Client, Deployment, StorageProviderConfig};
use boundless_market::deployments::NamedChain;
use risc0_zkvm::sha::{Digest, Digestible};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter};
use url::Url;
use warp::{Filter, Reply};

// 引入guest程序
risc0_zkvm::include_image!(pub RANDOM_NUMBER_ID, RANDOM_NUMBER_ELF, "random_number");

// 游戏状态结构
#[derive(Debug, Clone, Serialize)]
pub struct GameSession {
    pub game_id: String,
    pub random_number: Option<u32>,
    pub request_id: Option<String>,
    pub status: GameStatus,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub enum GameStatus {
    RequestingProof,
    WaitingForGuess,
    Completed { winner: bool, guess: u32 },
    Failed,
}

// API请求/响应结构
#[derive(Deserialize)]
struct CreateGameRequest {
    // 空结构体，不再需要player_address
}

#[derive(Serialize)]
struct CreateGameResponse {
    game_id: String,
    status: String,
}

#[derive(Deserialize)]
struct MakeGuessRequest {
    game_id: String,
    guess: u32,
    player_guess: String, // "higher" 或 "lower"
}

#[derive(Serialize)]
struct MakeGuessResponse {
    result: String,
    actual_number: u32,
    won: bool,
}

// 全局游戏状态存储
type GameStore = Arc<Mutex<HashMap<String, GameSession>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::from_str("info")?.into())
                .from_env_lossy(),
        )
        .init();

    // 游戏状态存储
    let games: GameStore = Arc::new(Mutex::new(HashMap::new()));

    // CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);

    // 静态文件服务
    let static_files = warp::path("static")
        .and(warp::fs::dir("./static"));

    // 主页
    let index = warp::path::end()
        .and(warp::fs::file("./static/index.html"));

    // API路由
    let create_game = warp::path!("api" / "create-game")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_games(games.clone()))
        .and_then(handle_create_game);

    let make_guess = warp::path!("api" / "guess")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_games(games.clone()))
        .and_then(handle_make_guess);

    let game_status = warp::path!("api" / "status" / String)
        .and(warp::get())
        .and(with_games(games.clone()))
        .and_then(handle_game_status);

    let routes = index
        .or(static_files)
        .or(create_game)
        .or(make_guess)
        .or(game_status)
        .with(cors);

    println!("🎲 猜数字游戏服务器启动在 http://localhost:3030");
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}

fn with_games(games: GameStore) -> impl Filter<Extract = (GameStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || games.clone())
}

async fn handle_create_game(
    req: CreateGameRequest,
    games: GameStore,
) -> Result<impl Reply, warp::Rejection> {
    let game_id = generate_game_id();
    let game_session = GameSession {
        game_id: game_id.clone(),
        random_number: None,
        request_id: None,
        status: GameStatus::RequestingProof,
        created_at: current_timestamp(),
    };

    // 存储游戏会话
    {
        let mut games = games.lock().unwrap();
        games.insert(game_id.clone(), game_session);
    }

    // 启动后台任务请求随机数证明
    let games_clone = games.clone();
    let game_id_clone = game_id.clone();
    tokio::spawn(async move {
        if let Err(e) = request_random_proof(game_id_clone, games_clone).await {
            tracing::error!("请求随机数证明失败: {}", e);
        }
    });

    Ok(warp::reply::json(&CreateGameResponse {
        game_id,
        status: "requesting_proof".to_string(),
    }))
}

async fn handle_make_guess(
    req: MakeGuessRequest,
    games: GameStore,
) -> Result<impl Reply, warp::Rejection> {
    let mut games = games.lock().unwrap();
    
    if let Some(game) = games.get_mut(&req.game_id) {
        if let Some(actual_number) = game.random_number {
            let won = match req.player_guess.as_str() {
                "higher" => req.guess > actual_number,
                "lower" => req.guess < actual_number,
                "equal" => req.guess == actual_number,
                _ => false,
            };

            game.status = GameStatus::Completed {
                winner: won,
                guess: req.guess,
            };

            Ok(warp::reply::json(&MakeGuessResponse {
                result: if won { "win".to_string() } else { "lose".to_string() },
                actual_number,
                won,
            }))
        } else {
            Ok(warp::reply::json(&MakeGuessResponse {
                result: "not_ready".to_string(),
                actual_number: 0,
                won: false,
            }))
        }
    } else {
        Ok(warp::reply::json(&MakeGuessResponse {
            result: "game_not_found".to_string(),
            actual_number: 0,
            won: false,
        }))
    }
}

async fn handle_game_status(
    game_id: String,
    games: GameStore,
) -> Result<impl Reply, warp::Rejection> {
    let games = games.lock().unwrap();
    
    if let Some(game) = games.get(&game_id) {
        Ok(warp::reply::json(&game))
    } else {
        Ok(warp::reply::json(&serde_json::json!({
            "error": "Game not found"
        })))
    }
}

async fn request_random_proof(game_id: String, games: GameStore) -> Result<()> {
    // 从环境变量读取配置
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://ethereum-sepolia-rpc.publicnode.com".to_string());
    let private_key = std::env::var("PRIVATE_KEY")
        .context("需要设置PRIVATE_KEY环境变量")?;

    let private_key: PrivateKeySigner = private_key.parse()
        .context("无效的私钥格式")?;

    // 使用Sepolia测试网的Boundless部署
    let deployment = Deployment::from_chain(NamedChain::Sepolia)
        .context("无法获取Sepolia部署配置")?;

    // 创建Boundless客户端
    let client = Client::builder()
        .with_rpc_url(Url::parse(&rpc_url)?)
        .with_private_key(private_key)
        .with_deployment(Some(deployment))
        .build()
        .await?;

    // 生成种子（使用当前时间戳）
    let seed = U256::from(current_timestamp());
    let input = seed.abi_encode();

    // 创建证明请求，设置合理的offer参数
    let request = client.new_request()
        .with_program(RANDOM_NUMBER_ELF)
        .with_stdin(&input)
        .with_offer(
            client.new_offer()
                .with_min_price(parse_ether("0.001")?)
                .with_max_price(parse_ether("0.002")?)
                .with_lock_stake(parse_ether("0.001")?)
                .with_lock_timeout(120) // 2分钟锁定超时
                .with_timeout(300) // 5分钟总超时
        );

    // 提交请求
    let (request_id, expires_at) = client.submit_onchain(request).await?;
    
    // 更新游戏状态
    {
        let mut games = games.lock().unwrap();
        if let Some(game) = games.get_mut(&game_id) {
            game.request_id = Some(format!("{:x}", request_id));
        }
    }

    tracing::info!("正在等待请求 {:x} 完成", request_id);

    // 等待证明完成
    let (journal, _seal) = client
        .wait_for_request_fulfillment(
            request_id,
            Duration::from_secs(5),
            expires_at,
        )
        .await?;

    // 解码随机数
    let random_number = U256::abi_decode(&journal, true)?;
    let random_number: u32 = random_number.try_into().unwrap_or(50);

    // 更新游戏状态
    {
        let mut games = games.lock().unwrap();
        if let Some(game) = games.get_mut(&game_id) {
            game.random_number = Some(random_number);
            game.status = GameStatus::WaitingForGuess;
        }
    }

    tracing::info!("游戏 {} 的随机数已生成: {}", game_id, random_number);

    Ok(())
}

fn generate_game_id() -> String {
    format!("game_{}", current_timestamp())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
} 