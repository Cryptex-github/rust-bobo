use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};

use songbird::{
    driver::{Config as DriverConfig, CryptoMode, DecodeMode},
    SerenityInit, Songbird,
};
use eval::Expr;

use serenity::framework::standard::Args;
use serenity::model::id::UserId;
use serenity::client::bridge::gateway::GatewayIntents;

use std::collections::hash_set::HashSet;
use std::time::Instant;
use std::env;

#[group]
#[commands(ping)]
struct General;

#[group]
#[commands(eval)]
struct Dev;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let mut owners = HashSet::new();
    owners.insert(UserId(590323594744168494));
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("rbobo ").owners(owners))
        .group(&GENERAL_GROUP)
        .group(&DEV_GROUP);
    
    let intents = GatewayIntents::all();
    
    let songbird = Songbird::serenity();
    songbird.set_config(
        DriverConfig::default()
            .decode_mode(DecodeMode::Decode)
            .crypto_mode(CryptoMode::Normal),
    );

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird_with(songbird)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let api_latency = {
        let instant = Instant::now();
        msg.channel_id.broadcast_typing(&ctx.http).await?;
        instant.elapsed().as_millis() as f64
    };
    msg.reply(ctx, format!("Pong :O API latency is {}", api_latency)).await?;

    Ok(())
}

#[command]
#[owners_only]
async fn eval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let result = Expr::new(args.rest()).value("msg", msg).exec().unwrap_or_default();
    msg.reply(ctx, format!("{:?}", result)).await?;
    
    Ok(())
}

