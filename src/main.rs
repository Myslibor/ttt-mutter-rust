use ::serenity::{Client, all::prelude::GatewayIntents, model::channel::Message};
use axum::{Form, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{Context, EditMember, EventHandler, GuildId, Http, Ready, UserId, prelude::TypeMapKey},
    async_trait,
};
use std::{collections::HashMap, io, path::Path, sync::Arc, thread};
use tokio::sync::RwLock;

mod commands;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    guild_id: u64,
    bot_token: String,
}

fn load_config() -> Config {
    if Path::new("config.json").exists() {
        let raw: String = std::fs::read_to_string("config.json").unwrap();
        let load: Config = serde_json::from_str(&raw).unwrap();
        return load;
    } else {
        let default = Config {
            guild_id: 0,
            bot_token: String::from("-"),
        };
        let raw = serde_json::to_string_pretty(&default).unwrap();
        std::fs::write("config.json", raw).unwrap();
        println!("Created default config");
        std::process::exit(0);
    }
}

type IdMap = HashMap<String, u64>;

fn load_id_map() -> IdMap {
    if Path::new("id_map.json").exists() {
        let raw: String = std::fs::read_to_string("id_map.json").unwrap();
        let result: IdMap;
        if raw.trim().is_empty() {
            result = IdMap::new();
        } else {
            result = serde_json::from_str(&raw).unwrap();
        }

        return result;
    } else {
        std::fs::File::create("id_map.json").unwrap();
        return IdMap::new();
    }
}

pub fn save_id_map(map: &IdMap) {
    let raw: String = serde_json::to_string_pretty(map).unwrap();
    if !raw.trim().is_empty() {
        std::fs::write("id_map.json", raw).unwrap();
    }
}

#[derive(Clone)]
struct AppState {
    id_map: Arc<RwLock<IdMap>>,
    guild_id: GuildId,
    http: Arc<Http>,
}

struct AppStateKey;
impl TypeMapKey for AppStateKey {
    type Value = Arc<AppState>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("Logged in as {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let data = ctx.data.read().await;
        let app_data = data.get::<AppStateKey>().unwrap().clone();
        drop(data);

        let guild_should = app_data.guild_id.get();
        let guild_now = msg.guild_id.unwrap().get();

        if guild_now != guild_should {
            return;
        }

        commands::handle(&ctx, &msg, &app_data).await;
    }
}

#[derive(Deserialize)]
struct SteamForm {
    steamid: Option<String>,
}

async fn player_death(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SteamForm>,
) -> (StatusCode, String) {
    let steam_id_opt = form.steamid;
    if steam_id_opt.is_none() {
        return (StatusCode::BAD_REQUEST, "Missing steamid".into());
    }
    let steam_id = steam_id_opt.unwrap();

    let discord_id_opt = state.id_map.read().await.get(&steam_id).copied();
    if discord_id_opt.is_none() {
        println!("No Discord mapping for SteamID64 {steam_id}");
        return (StatusCode::BAD_REQUEST, "No mapping".into());
    }
    let discord_id = discord_id_opt.unwrap();

    set_mute(&state, UserId::new(discord_id), true).await;
    return (StatusCode::ACCEPTED, "OK".into());
}

async fn player_res(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SteamForm>,
) -> (StatusCode, String) {
    let steam_id_opt = form.steamid;
    if steam_id_opt.is_none() {
        return (StatusCode::BAD_REQUEST, "Missing steamid".into());
    }
    let steam_id = steam_id_opt.unwrap();

    let discord_id_opt = state.id_map.read().await.get(&steam_id).copied();
    if discord_id_opt.is_none() {
        println!("No Discord mapping for SteamID64 {steam_id}");
        return (StatusCode::BAD_REQUEST, "No mapping".into());
    }
    let discord_id = discord_id_opt.unwrap();

    set_mute(&state, UserId::new(discord_id), true).await;
    return (StatusCode::ACCEPTED, "OK".into());
}

async fn handle_new_round(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
    let map = state.id_map.read().await;
    for (_, discord_id) in map.iter() {
        set_mute(&state, UserId::new(*discord_id), false).await;
    }

    return (StatusCode::ACCEPTED, "OK".into());
}

async fn set_mute(state: &AppState, user: UserId, mute: bool) {
    let edit = state
        .guild_id
        .edit_member(&state.http, user, EditMember::new().mute(mute))
        .await;
    if edit.is_ok() {
        let change = if mute { "Muted" } else { "Unmuted" };
        println!("{change} {}", edit.unwrap().display_name());
    }
}

fn console(id_map: Arc<RwLock<IdMap>>) {
    loop {
        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd).unwrap();
        cmd = cmd.trim().into();

        if cmd == "stop" {
            println!("Shutting down");
            let map = id_map.blocking_read();
            save_id_map(&map);
            std::process::exit(0);
        }
    }
}

#[tokio::main]
async fn main() {
    let config = load_config();
    let id_map = load_id_map();

    let app_data = Arc::new(AppState {
        id_map: Arc::new(RwLock::new(id_map)),
        guild_id: GuildId::new(config.guild_id),
        http: Arc::new(Http::new(&config.bot_token)),
    });

    println!(
        "GUILD_ID set to {} and BOT_TOKEN set to {}",
        config.guild_id, config.bot_token
    );

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(config.bot_token, intents)
        .event_handler(Handler)
        .await
        .expect("create client");

    let app: Router = Router::new()
        .route("/death", post(player_death))
        .route("/res", post(player_res))
        .route("/newround", post(handle_new_round))
        .with_state(app_data.clone());

    let console_id_map = app_data.id_map.clone();

    client
        .data
        .write()
        .await
        .insert::<AppStateKey>(app_data.clone());

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:5003").await.unwrap();
        println!("HTTP server listening on 0.0.0.0:5003");
        axum::serve(listener, app).await.unwrap();
    });

    thread::spawn(move || {
        console(console_id_map);
    });

    if let Err(e) = client.start().await {
        eprintln!("Client error: {e}");
    }
}
