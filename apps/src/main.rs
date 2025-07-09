use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH}
};

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter};
use warp::{Filter, Reply};

use tokio::process::Command;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

// 引入guest程序

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
        .run(([0, 0, 0, 0], 3030))
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
        status: GameStatus::WaitingForGuess,
        created_at: current_timestamp(),
    };

    // 存储游戏会话
    {
        let mut games = games.lock().unwrap();
        games.insert(game_id.clone(), game_session);
    }

    // 直接本地生成随机数
    let random_number = generate_random_number();

    {
        let mut games = games.lock().unwrap();
        if let Some(game) = games.get_mut(&game_id) {
            game.random_number = Some(random_number);
            game.status = GameStatus::WaitingForGuess;
        }
    }

    Ok(warp::reply::json(&CreateGameResponse {
        game_id,
        status: "ready".to_string(),
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

            // 异步提交证明
            let rn = actual_number;
            let gs = req.guess;
            tokio::spawn(async move {
                if let Err(e) = spawn_cli_proof(rn, gs, won).await {
                    tracing::error!("提交游戏结果证明失败: {}", e);
                }
            });

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

fn generate_game_id() -> String {
    format!("game_{}", current_timestamp())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 生成 1~100 的随机数
fn generate_random_number() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=100)
}

/// 调用 boundless CLI 提交证明请求（后台运行，不关心结果）
async fn spawn_cli_proof(random_number: u32, guess: u32, won: bool) -> Result<()> {
    // 如果没有配置 RPC_URL / PRIVATE_KEY 则直接返回
    let rpc_url = match std::env::var("RPC_URL") {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("未设置 RPC_URL，跳过提交证明");
            return Ok(());
        }
    };
    let private_key = match std::env::var("PRIVATE_KEY") {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("未设置 PRIVATE_KEY，跳过提交证明");
            return Ok(());
        }
    };

    // 找到编译好的 ELF 路径 (release, riscv target)
    let mut elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // 使用 cargo risczero build 导出的 bin 文件
    elf_path.push("../target/riscv32im-risc0-zkvm-elf/docker/game-result.bin");

    if !elf_path.exists() {
        tracing::error!("找不到 game-result ELF，请先执行 'cargo build --release --target riscv32im-risc0_zkvm-elf'");
        return Ok(());
    }

    let elf_bytes = fs::read(&elf_path)?;

    // 将 ELF 拷贝到临时文件，供 CLI 使用
    let mut program_path = std::env::temp_dir();
    program_path.push(format!("game_result_{}.elf", current_timestamp()));
    fs::write(&program_path, &elf_bytes)?;

    // 构造输入 bytes: random_number(u32 LE) | guess(u32 LE) | won(u8)
    let mut input_bytes = Vec::with_capacity(9);
    input_bytes.extend(&random_number.to_le_bytes());
    input_bytes.extend(&guess.to_le_bytes());
    input_bytes.push(if won { 1 } else { 0 });

    // 写入临时文件，供 CLI 作为 --input-file 使用
    let mut input_path = std::env::temp_dir();
    input_path.push(format!("input_{}.bin", current_timestamp()));
    fs::write(&input_path, &input_bytes)?;

    // 组装 CLI 命令：使用 submit-offer 子命令，并通过本地文件提供 program 与 input
    let mut cmd = Command::new("boundless");
    cmd.arg("--rpc-url").arg(&rpc_url);
    cmd.arg("--private-key").arg(&private_key);

    cmd.args([
        "request",
        "submit-offer",
        "--program",
        program_path.to_string_lossy().as_ref(),
        "--input-file",
        input_path.to_string_lossy().as_ref(),
        "--encode-input",
        // 如无明确定价需求，使用 CLI 内部默认报价
    ]);

    // 后台运行，不等待结果
    cmd.spawn()?;

    Ok(())
} 