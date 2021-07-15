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

use songbird::SerenityInit;
use eval::Expr;

use serenity::framework::standard::Args;
use serenity::model::id::UserId;
use serenity::client::bridge::gateway::GatewayIntents;

use std::collections::hash_set::HashSet;
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

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird()
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong :O").await?;

    Ok(())
}

#[command]
#[owners_only]
async fn eval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let result = Expr::new(args.rest()).value("msg", msg).exec().unwrap_or_default();
    msg.reply(ctx, format!("{:?}", result)).await?;
    
    Ok(())
}

