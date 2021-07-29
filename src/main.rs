use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::prelude::TypeMapKey;
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};
use serenity::{
    model::{
        channel::Message,
        id::ChannelId,
        misc::Mentionable
    },
};

use songbird::{
    driver::{Config as DriverConfig, CryptoMode, DecodeMode},
    SerenityInit, Songbird,
    model::payload::{ClientConnect, ClientDisconnect, Speaking},
    CoreEvent,
    Event,
    EventContext,
    EventHandler as VoiceEventHandler,
};
use eval::Expr;

use reqwest;

use image::ColorType;
use image::codecs::png::PngEncoder;
use image::EncodableLayout;

use sqlx;
use sqlx::postgres::{PgPool, PgPoolOptions};

use photon_rs::PhotonImage;
use photon_rs::native::image_to_bytes;
use photon_rs::native::open_image_from_bytes;
use photon_rs::channels::invert as photon_invert;
use photon_rs::multiple::apply_gradient;
use photon_rs::filters::filter;

use serenity::framework::standard::Args;
use serenity::model::id::UserId;
use serenity::client::bridge::gateway::GatewayIntents;

use std::io::Cursor;
use std::path::Path;
use std::collections::hash_set::HashSet;
use std::time::Instant;
use std::env;


struct Pool;

impl TypeMapKey for Pool {
    type Value = PgPool;
}


#[group]
#[commands(ping)]
struct General;

#[group]
#[commands(invert, rainbow, oceanic, islands, marine, seagreen, flagblue, liquid, diamante, radio, twenties, rosetint, mauve, bluechrome, vintage, perfume)]
struct Image;

#[group]
#[commands(eval)]
struct Dev;

#[group]
#[commands(join, leave)]
struct Voice;

struct Receiver;

impl Receiver {
    pub fn new() -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        Self { }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[async_trait]
impl VoiceEventHandler for Receiver {
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;
        match ctx {
            Ctx::SpeakingStateUpdate(
                Speaking {speaking, ssrc, user_id, ..}
            ) => {
                // Discord voice calls use RTP, where every sender uses a randomly allocated
                // *Synchronisation Source* (SSRC) to allow receivers to tell which audio
                // stream a received packet belongs to. As this number is not derived from
                // the sender's user_id, only Discord Voice Gateway messages like this one
                // inform us about which random SSRC a user has been allocated. Future voice
                // packets will contain *only* the SSRC.
                //
                // You can implement logic here so that you can differentiate users'
                // SSRCs and map the SSRC to the User ID and maintain this state.
                // Using this map, you can map the `ssrc` in `voice_packet`
                // to the user ID and handle their audio packets separately.
                println!(
                    "Speaking state update: user {:?} has SSRC {:?}, using {:?}",
                    user_id,
                    ssrc,
                    speaking,
                );
            },
            Ctx::SpeakingUpdate {ssrc, speaking} => {
                // You can implement logic here which reacts to a user starting
                // or stopping speaking.
                println!(
                    "Source {} has {} speaking.",
                    ssrc,
                    if *speaking {"started"} else {"stopped"},
                );
            },
            Ctx::VoicePacket {audio, packet, payload_offset, payload_end_pad} => {
                // An event which fires for every received audio packet,
                // containing the decoded data.
                if let Some(audio) = audio {
                    // println!("Audio packet's first 5 samples: {:?}", audio.get(..5.min(audio.len())));
                    // println!(
                    //    "Audio packet sequence {:05} has {:04} bytes (decompressed from {}), SSRC {}",
                    //    packet.sequence.0,
                    //    audio.len() * std::mem::size_of::<i16>(),
                    //    packet.payload.len(),
                    //    packet.ssrc,
                    // );
                } else {
                    println!("RTP packet, but no audio. Driver may not be configured to decode.");
                }
            },
            Ctx::RtcpPacket {packet, payload_offset, payload_end_pad} => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                // println!("RTCP packet received: {:?}", packet);
            },
            Ctx::ClientConnect(
                ClientConnect {audio_ssrc, video_ssrc, user_id, ..}
            ) => {
                // You can implement your own logic here to handle a user who has joined the
                // voice channel e.g., allocate structures, map their SSRC to User ID.

                println!(
                    "Client connected: user {:?} has audio SSRC {:?}, video SSRC {:?}",
                    user_id,
                    audio_ssrc,
                    video_ssrc,
                );
            },
            Ctx::ClientDisconnect(
                ClientDisconnect {user_id, ..}
            ) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // speaking or connecting.

                println!("Client disconnected: user {:?}", user_id);
            },
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut owners = HashSet::new();
    owners.insert(UserId(590323594744168494));
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("rbobo ").owners(owners))
        .group(&GENERAL_GROUP)
        .group(&DEV_GROUP)
        .group(&VOICE_GROUP)
        .group(&IMAGE_GROUP);
    
    let intents = GatewayIntents::all();
    
    let songbird = Songbird::serenity();
    songbird.set_config(
        DriverConfig::default()
            .decode_mode(DecodeMode::Decode)
            .crypto_mode(CryptoMode::Normal),
    );
    
    let pool = PgPoolOptions::new()
        .max_connections(10)
//         .socket(Path::new("/var/run/postgresql").as_ref())
//         .user("postgres1")
//         .password("postgres")
//         .database("rustbobo")
        // "postgresql://postgres1:postgres@/var/run/postgresql:5432/rustbobo"
        .connect("postgresql://postgres1:postgres@/var/run/postgresql:5432/rustbobo").await;

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .intents(intents)
        .register_songbird_with(songbird)
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;
        
        data.insert::<Pool>(pool);
    }
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
//     let pool = {
//         let data_read = ctx.data.read().await;
//         data_read.get::<Pool>().unwrap().clone()
//     };
//     let db_latency = {
//         let instant = Instant::now();
//         sqlx::query!("SELECT 1").execute(&pool).await?;
//         instant.elapsed().as_millis() as f64
//     };
        
    msg.reply(ctx, format!("Pong :O API latency is {}", api_latency)).await?;

    Ok(())
}


async fn manip_filter_image(msg: &Message, ctx: &Context, filter_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let instant = Instant::now();
    let avatar_url = msg.author.face().replace(".webp", ".png");
    let content = reqwest::get(avatar_url).await?.bytes().await?;
    let mut image = open_image_from_bytes(&content).unwrap();
    filter(&mut image, filter_name);
    let mut buffer = Cursor::new(vec![]);
    let encoder = PngEncoder::new(&mut buffer);
    let width = image.get_width();
    let height = image.get_height();
    let _ = encoder.encode(image_to_bytes(image).as_bytes(), width, height, ColorType::Rgba8);
    let encoded_image = buffer.into_inner();
    let files = vec![(encoded_image.as_bytes(), "rustbobo_image_manip.png")];
    msg.channel_id.send_files(&ctx.http, files, |m| m.content(format!("Process Time: {} ms", instant.elapsed().as_millis()))).await?;
    
    Ok(())
}

async fn manip_image<T>(msg: &Message, ctx: &Context, photon_function: T) -> Result<(), Box<dyn std::error::Error>> 
    where T: Fn(&mut PhotonImage) -> () {
    let instant = Instant::now();
    let avatar_url = msg.author.face().replace(".webp", ".png");
    let content = reqwest::get(avatar_url).await?.bytes().await?;
    let mut image = open_image_from_bytes(&content).unwrap();
    photon_function(&mut image);
    let mut buffer = Cursor::new(vec![]);
    let encoder = PngEncoder::new(&mut buffer);
    let width = image.get_width();
    let height = image.get_height();
    let _ = encoder.encode(image_to_bytes(image).as_bytes(), width, height, ColorType::Rgba8);
    let encoded_image = buffer.into_inner();
    let files = vec![(encoded_image.as_bytes(), "rustbobo_image_manip.png")];
    msg.channel_id.send_files(&ctx.http, files, |m| m.content(format!("Process Time: {} ms", instant.elapsed().as_millis()))).await?;
    
    Ok(())
}

#[command]
async fn perfume(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "perfume").await;
    
    Ok(())
}

#[command]
async fn vintage(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "vintage").await;
    
    Ok(())
}

#[command]
async fn bluechrome(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "bluechrome").await;
    
    Ok(())
}

#[command]
async fn mauve(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "mauve").await;
    
    Ok(())
}

#[command]
async fn rosetint(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "rosetint").await;
    
    Ok(())
}

#[command]
async fn twenties(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "twenties").await;
    
    Ok(())
}

#[command]
async fn radio(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "radio").await;
    
    Ok(())
}

#[command]
async fn diamante(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "diamante").await;
    
    Ok(())
}

#[command]
async fn liquid(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "liquid").await;
    
    Ok(())
}

#[command]
async fn flagblue(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "flagblue").await;
    
    Ok(())
}

#[command]
async fn seagreen(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "seagreen").await;
    
    Ok(())
}

#[command]
async fn marine(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "marine").await;
    
    Ok(())
}

#[command]
async fn islands(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "islands").await;
    
    Ok(())
}

#[command]
async fn oceanic(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_filter_image(msg, ctx, "oceanic").await;
    
    Ok(())
}

#[command]
async fn invert(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_image(msg, ctx, photon_invert).await;
    
    Ok(())
}
    
#[command]
async fn rainbow(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = manip_image(msg, ctx, apply_gradient).await;
    
    Ok(())
}

#[command]
#[owners_only]
async fn eval(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let result = Expr::new(args.rest()).value("msg", msg).exec().unwrap_or_default();
    msg.reply(ctx, format!("{:?}", result)).await?;
    
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let connect_to = match args.single::<u64>() {
        Ok(id) => ChannelId(id),
        Err(_) => {
            msg.reply(ctx, "Requires a valid voice channel ID be given").await?;

            return Ok(());
        },
    };

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    let (handler_lock, conn_result) = manager.join(guild_id, connect_to).await;

    if let Ok(_) = conn_result {
        // NOTE: this skips listening for the actual connection result.
        let mut handler = handler_lock.lock().await;

        handler.add_global_event(
            CoreEvent::SpeakingStateUpdate.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::SpeakingUpdate.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::VoicePacket.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::RtcpPacket.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::ClientConnect.into(),
            Receiver::new(),
        );

        handler.add_global_event(
            CoreEvent::ClientDisconnect.into(),
            Receiver::new(),
        );

        msg.channel_id.say(&ctx.http, &format!("Joined {}", connect_to.mention())).await?;
    } else {
        msg.channel_id.say(&ctx.http, "Error joining the channel").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await?;
        }

        msg.channel_id.say(&ctx.http,"Left voice channel").await?;
    } else {
        msg.reply(ctx, "Not in a voice channel").await?;
    }

    Ok(())
}

