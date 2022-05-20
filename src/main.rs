use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::time::Duration;

use dotenv::dotenv;
use rand::Rng;
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, DispatchError, HelpOptions,
};
use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::UserId;
use serenity::prelude::{Context, EventHandler, GatewayIntents};
use serenity::Client;

struct Handler;

const PREFIX: &str = "m!";

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let bot_id = ctx.cache.current_user_id();
        // println!("{}", bot_id);

        if msg.content == format!("<@{}>", bot_id) || msg.content == format!("<@!{}>", bot_id) {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, format!("Prefix is `{}`", PREFIX))
                .await
            {
                println!(
                    "Error in {} - {}: {:?}",
                    msg.channel_id.name(&ctx.cache).await.unwrap(),
                    msg.channel_id,
                    why
                )
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} - {} is ready", ready.user.name, ready.user.id)
    }
}

#[hook]
async fn handle_errors(ctx: &Context, msg: &Message, error: DispatchError, command: &str) {
    match error {
        DispatchError::NotEnoughArguments { min, given } => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Not enough arguments in {}!\nminimum required {}/supplied arguments {}",
                        command, min, given
                    ),
                )
                .await
                .expect("Could not send message");
        }
        err => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Unhandled error in {}: {:?}", command, err),
                )
                .await
                .expect("Could not send message");
        }
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx, "Pong!").await?;
    Ok(())
}

#[command]
async fn ngg(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx, "Guess the number!").await?;
    let secret_number = rand::thread_rng().gen_range(1..101);
    let mut attempts = 0;
    let max_attempts = 7;
    let game_end_keywords = ["cancel", "stop", "quit", "exit"];

    loop {
        msg.channel_id.say(&ctx, "Please send your guess").await?;

        let mut stop_game = false;

        let guess = match msg
            .author
            .await_reply(&ctx)
            .channel_id(msg.channel_id)
            .filter(move |m| {
                return m.content.chars().all(char::is_numeric)
                    || game_end_keywords.contains(&&*m.content.to_lowercase());
            })
            .timeout(Duration::from_secs(15))
            .await
        {
            Some(answer) => {
                // println!("{:?}", answer);
                answer.content.clone()
            }
            None => {
                msg.channel_id
                    .say(&ctx, "You ran out of time! Game over!")
                    .await?;
                break;
            }
        };

        if game_end_keywords.contains(&&*guess.to_lowercase()) {
            msg.channel_id.say(&ctx, "Game over!").await?;
            break;
        }

        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                msg.channel_id
                    .say(&ctx, "An error occurred, send another **number.**")
                    .await?;
                continue;
            }
        };

        attempts += 1;

        let mut to_send = format!("You guessed: {}", guess);

        match guess.cmp(&secret_number) {
            Ordering::Less => to_send += "\nToo small!",

            Ordering::Greater => to_send += "\nToo big!",

            Ordering::Equal => {
                to_send += "\nYou win!";
                // msg.channel_id.say(&ctx.http, "You win!").await?;
                stop_game = true;
            }
        };

        if attempts == max_attempts {
            stop_game = if !stop_game {
                to_send += "\nYou're out of attempts!";
                stop_game = true;
                stop_game
            } else {
                stop_game
            }
        } else {
            to_send += &*format!("\n{}/{}", attempts, max_attempts);
            // msg.channel_id.say(&ctx.http, format!("{}/{}", attempts, max_attempts)).await?;
        }
        msg.channel_id.say(ctx, to_send).await?;

        if stop_game {
            break;
        }
    }

    Ok(())
}

#[command]
#[min_args(1)]
#[only_in("guilds")]
#[owners_only]
async fn nick(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.message().to_lowercase() == "reset" {
        msg.guild_id
            .unwrap()
            .edit_nickname(&ctx.http, None)
            .await
            .expect("Could not reset nickname");

        msg.reply_ping(&ctx.http, "Reset my nickname")
            .await
            .expect("Could not send message");

        return Ok(());
    }

    msg.guild_id
        .unwrap()
        .edit_nickname(&ctx.http, Option::from(args.message()))
        .await
        .expect("Could not change nickname");

    msg.reply_ping(
        &ctx.http,
        format!("Changed my nickname to `\"{}\"`", args.message()),
    )
    .await
    .expect("Could not send message");
    Ok(())
}

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(ngg, ping, nick)]
struct Uncategorized;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let owners_list = [UserId::from(352216606098587650)];
    let owners: HashSet<UserId> = HashSet::from(owners_list);
    let token = env::var("TOKEN").expect("No token in env var");
    let intents = GatewayIntents::all();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix(PREFIX).owners(owners).ignore_bots(true))
        .group(&UNCATEGORIZED_GROUP)
        .help(&MY_HELP)
        .on_dispatch_error(handle_errors);

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error making client");

    if let Err(why) = client.start().await {
        println!("client error: {:?}", why)
    }
}
