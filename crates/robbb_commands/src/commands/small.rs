use chrono::Utc;
use robbb_db::fetch_field::FetchField;

use super::*;

/// Restart robbb
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn restart(ctx: Ctx<'_>) -> Res<()> {
    let _ = ctx.say_success("Shutting down").await;
    ctx.discord().shard.shutdown_clean();
    std::process::exit(1);
}

/// Make the bot say something. Please don't actually use this :/
#[poise::command(
    slash_command,
    guild_only,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn say(
    ctx: Ctx<'_>,
    #[description = "What you,.. ummmm. I mean _I_ should say"] message: String,
) -> Res<()> {
    ctx.send(|m| m.content("Sure thing!").ephemeral(true)).await?;
    ctx.channel_id().say(&ctx.discord(), message).await?;
    Ok(())
}

/// Get some latency information
#[poise::command(
    prefix_command,
    slash_command,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn latency(ctx: Ctx<'_>) -> Res<()> {
    let shard_latency = {
        let shard_manager = ctx.framework().shard_manager.as_ref().lock().await;
        let shard_runners = shard_manager.runners.lock().await;
        shard_runners.values().find_map(|runner| runner.latency)
    };

    let msg_latency = match ctx {
        poise::Context::Application(_) => None,
        poise::Context::Prefix(prefix_ctx) => {
            let msg_time = prefix_ctx.msg.timestamp;
            let now = Utc::now();
            Some(std::time::Duration::from_millis(
                (now.timestamp_millis() - msg_time.timestamp_millis()).unsigned_abs(),
            ))
        }
    };

    ctx.send_embed(|e| {
        e.title("Latency information");
        if let Some(latency) = shard_latency {
            e.field(
                "Shard latency (last heartbeat send → ACK receive)",
                humantime::Duration::from(latency),
                false,
            );
        }
        if let Some(latency) = msg_latency {
            e.field(
                "Message latency (message timestamp → message received)",
                humantime::Duration::from(latency),
                false,
            );
        }
    })
    .await?;

    Ok(())
}

/// I'm tired,... >.<
#[poise::command(prefix_command, slash_command, category = "Bot-Administration")]
pub async fn uptime(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();
    ctx.send_embed(|e| {
        e.title("Uptime");
        e.description(format!("Started {}", util::format_date_detailed(config.time_started)));
    })
    .await?;
    Ok(())
}

/// Send a link to the bot's repository! Feel free contribute!
#[poise::command(prefix_command, slash_command)]
pub async fn repo(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://github.com/unixporn/robbb").await?;
    Ok(())
}

/// Get the invite to the unixporn discord server
#[poise::command(prefix_command, slash_command)]
pub async fn invite(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://discord.gg/4M7SYzn3BW").await?;
    Ok(())
}

/// Get a users description. Provide your own using /setfetch.
#[poise::command(prefix_command, guild_only, slash_command)]
pub async fn description(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<Member>,
) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(desc) = fetch.and_then(|x| x.info.get(&FetchField::Description).cloned()) {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Description");
            e.description(desc);
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't set their description", user.user.tag())).await?;
    }
    Ok(())
}
/// Get a users dotfiles. Provide your own using /setfetch.
#[poise::command(prefix_command, guild_only, slash_command)]
pub async fn dotfiles(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(dots) = fetch.and_then(|x| x.info.get(&FetchField::Dotfiles).cloned()) {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Dotfiles");
            e.description(dots);
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't provided their dotfiles", user.user.tag())).await?;
    }
    Ok(())
}

/// Get a users git profile. Provide your own using /setfetch.
#[poise::command(prefix_command, guild_only, slash_command)]
pub async fn git(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(git) = fetch.and_then(|x| x.info.get(&FetchField::Git).cloned()) {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Git profile");
            e.description(git);
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't provided their git profile", user.user.tag())).await?;
    }
    Ok(())
}
