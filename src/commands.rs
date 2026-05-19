use serenity::{all::prelude::Context, model::channel::Message};

use crate::{AppState, save_id_map};

pub async fn handle(ctx: &Context, msg: &Message, data: &AppState) {
    if msg.content == "!tttbot_map_reset" {
        tttbot_map_reset(ctx, msg, data).await;
    } else if msg.content.starts_with("!tttbot_map ") {
        tttbot_map(ctx, msg, data).await;
    } else if msg.content == "!tttbot_end" {
        tttbot_end(ctx, msg, data).await;
    } else if msg.content.starts_with("!tttbot_map_dc ") {
        tttbot_map_dc(ctx, msg, data).await;
    } else if msg.content.starts_with("!tttbot_map_delete ") {
        tttbot_map_delete(ctx, msg, data).await;
    } else if msg.content == "!tttbot_map_print" {
        tttbot_map_print(ctx, msg, data).await;
    }
}

async fn tttbot_map(ctx: &Context, msg: &Message, data: &AppState) {
    let steam_id = msg
        .content
        .strip_prefix("!tttbot_map ")
        .unwrap()
        .trim()
        .to_string();

    let discord_id = msg.author.id.get();

    let mut map = data.id_map.write().await;
    map.insert(steam_id.clone(), discord_id);

    msg.channel_id
        .say(
            &ctx.http,
            format!("Mapped SteamID64 {steam_id} to Discord_ID {discord_id}"),
        )
        .await
        .unwrap();

    save_id_map(&map);
}

async fn tttbot_map_dc(ctx: &Context, msg: &Message, data: &AppState) {
    let args = msg
        .content
        .strip_prefix("!tttbot_map_dc ")
        .unwrap()
        .trim()
        .to_string();
    let mut args_split = args.split_whitespace();

    if args_split.clone().count() != 2 {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Both the SteamID64 and Discord_ID must be  provided"),
            )
            .await
            .unwrap();
    }

    let steam_id: String = args_split.next().unwrap().into();
    let discord_id_string: String = args_split.next().unwrap().into();
    let discord_id = discord_id_string.parse::<u64>().unwrap();

    let mut map = data.id_map.write().await;
    map.insert(steam_id.clone(), discord_id);

    msg.channel_id
        .say(
            &ctx.http,
            format!("Mapped SteamID64 {steam_id} to Discord_ID {discord_id}"),
        )
        .await
        .unwrap();

    save_id_map(&map);
}

async fn tttbot_map_delete(ctx: &Context, msg: &Message, data: &AppState) {
    let steam_id = msg
        .content
        .strip_prefix("!tttbot_map_delete ")
        .unwrap()
        .trim()
        .to_string();

    let mut map = data.id_map.write().await;
    if !map.contains_key(&steam_id) {
        msg.channel_id
            .say(&ctx.http, format!("No SteamID64 {steam_id} in id map"))
            .await
            .unwrap();
        return;
    }

    map.remove(&steam_id);

    msg.channel_id
        .say(
            &ctx.http,
            format!("Removed SteamID64 {steam_id} from id map"),
        )
        .await
        .unwrap();

    save_id_map(&map);
}

async fn tttbot_end(ctx: &Context, msg: &Message, data: &AppState) {
    msg.channel_id
        .say(&ctx.http, format!("Shutting down"))
        .await
        .unwrap();

    let map = data.id_map.write().await;
    save_id_map(&map);
    std::process::exit(0);
}

async fn tttbot_map_reset(ctx: &Context, msg: &Message, data: &AppState) {
    let mut map = data.id_map.write().await;
    map.clear();
    save_id_map(&map);

    msg.channel_id
        .say(&ctx.http, format!("Reset the id map"))
        .await
        .unwrap();
}

async fn tttbot_map_print(ctx: &Context, msg: &Message, data: &AppState) {
    let map = data.id_map.read().await;

    if map.is_empty() {
        msg.channel_id
            .say(&ctx.http, format!("No mappings found"))
            .await
            .unwrap();

        return;
    }

    let mut to_send: String = "".into();
    for (steam_id, discord_id) in map.iter() {
        to_send = format!(
            "{}SteamID64({steam_id}) -> Discord_ID({discord_id})\n",
            to_send
        )
    }

    if to_send.len() > 1900 {
        for line in to_send.split("\n") {
            msg.channel_id
                .say(&ctx.http, format!("{}", line))
                .await
                .unwrap();
        }
    } else {
        msg.channel_id
            .say(&ctx.http, format!("{}", to_send))
            .await
            .unwrap();
    }
}
