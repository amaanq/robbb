use poise::{
    serenity_prelude::{MemberParseError, UserParseError},
    TooFewArguments, TooManyArguments,
};
use robbb_commands::commands;

use robbb_util::{
    extensions::PoiseContextExt,
    log_error,
    prelude::{self, Ctx},
    util, UserData,
};

/// Handler passed to poise
pub async fn on_error(error: poise::FrameworkError<'_, UserData, prelude::Error>) {
    use poise::FrameworkError::*;
    match error {
        Command { error, ctx } => {
            handle_command_error(ctx, error).await;
        }
        Setup { error } => {
            tracing::error!(error = %error, "Error during setup: {}", error)
        }
        Listener { error, event, ctx: _, framework: _ } => {
            tracing::error!(event = ?event, error = %error, "Error in event listener: {}", error);
        }
        ArgumentParse { input, ctx, error } => {
            log_error!(handle_argument_parse_error(ctx, error, input).await);
        }
        CommandStructureMismatch { description, ctx } => {
            log_error!(poise::Context::Application(ctx).say_error("Something went wrong").await);
            tracing::error!(error="CommandStructureMismach", error.description=%description, "Error in command structure: {}", description);
        }
        CooldownHit { remaining_cooldown, ctx } => log_error!(
            ctx.say_error(format!(
                "You're doing this too much. Try again {}",
                util::format_date_ago(util::time_after_duration(remaining_cooldown))
            ))
            .await
        ),
        MissingBotPermissions { missing_permissions, ctx } => {
            log_error!(
                ctx.say_error(format!(
                    "It seems like I am lacking the {} permission",
                    missing_permissions
                ))
                .await
            );
            tracing::error!(
                error = "Missing permissions",
                "Bot missing permissions: {}",
                missing_permissions
            )
        }
        MissingUserPermissions { missing_permissions, ctx } => {
            log_error!(ctx.say_error("Missing permissions").await);
            tracing::error!(
                error = "User missing permissions",
                error.missing_permissions = ?missing_permissions,
                "User missing permissions: {:?}",
                missing_permissions
            )
        }
        NotAnOwner { ctx } => {
            log_error!(ctx.say_error("You need to be an owner to do this").await);
        }
        GuildOnly { ctx } => {
            log_error!(ctx.say_error("This can only be ran in a server").await);
        }
        DmOnly { ctx } => {
            log_error!(ctx.say_error("This can only be used in DMs").await);
        }
        NsfwOnly { ctx } => {
            log_error!(ctx.say_error("This can only be used in NSFW channels").await);
        }
        CommandCheckFailed { error, ctx } => {
            if let Some(error) = error {
                log_error!(
                    ctx.say_error("Something went wrong while checking your permissions").await
                );
                tracing::error!(
                    error = %error,
                    command_name = %ctx.command().qualified_name.as_str(),
                    "Error while running command check: {}", error
                );
            } else if matches!(ctx, poise::Context::Application(_)) {
                log_error!(
                    ctx.send(|m| m.ephemeral(true).content("Insufficient permissions")).await
                );
            }
        }
        DynamicPrefix { error } => {
            tracing::error!(error = %error, "Error in dynamic prefix");
        }
        other => {
            tracing::error!(error = ?other, "unhandled error received from poise");
        }
    }
}

async fn handle_argument_parse_error(
    ctx: Ctx<'_>,
    error: Box<dyn std::error::Error + Send + Sync>,
    input: Option<String>,
) -> anyhow::Result<()> {
    let msg = if error.downcast_ref::<humantime::DurationError>().is_some() {
        format!("'{}' is not a valid duration", input.unwrap_or_default())
    } else if error.downcast_ref::<UserParseError>().is_some() {
        format!("I couldn't find any user '{}'", input.unwrap_or_default())
    } else if error.downcast_ref::<MemberParseError>().is_some() {
        format!("I couldn't find any member '{}'", input.unwrap_or_default())
    } else if error.downcast_ref::<TooManyArguments>().is_some() {
        "Too many arguments".to_string()
    } else if error.downcast_ref::<TooFewArguments>().is_some() {
        "Too few arguments".to_string()
    } else if let Some(input) = input {
        format!("Malformed argument '{}'", input)
    } else {
        "Command used incorrectly".to_string()
    };
    ctx.say_error(msg).await?;
    Ok(())
}

async fn handle_command_error(ctx: Ctx<'_>, err: prelude::Error) {
    match err.downcast_ref::<commands::UserErr>() {
        Some(err) => match err {
            commands::UserErr::MentionedUserNotFound => {
                let _ = ctx.say_error("No user found with that name").await;
            }
            commands::UserErr::Other(issue) => {
                let _ = ctx.say_error(format!("Error: {}", issue)).await;
                tracing::info!(
                    user_error.message=%issue,
                    user_error.command_name = %ctx.command().qualified_name.as_str(),
                    "User error"
                );
            }
        },
        None => match err.downcast::<serenity::Error>() {
            Ok(err) => {
                tracing::warn!(
                    error.command_name = %ctx.command().qualified_name.as_str(),
                    error.message = %err,
                    "Serenity error [handling {}]: {} ({:?})",
                    ctx.command().name,
                    &err,
                    &err
                );
                match err {
                    serenity::Error::Http(err) => {
                        if let serenity::http::error::Error::UnsuccessfulRequest(res) = *err {
                            if res.status_code == serenity::http::StatusCode::NOT_FOUND
                                && res.error.message.to_lowercase().contains("unknown user")
                            {
                                let _ = ctx.say_error("User not found").await;
                            } else {
                                let _ = ctx.say_error("Something went wrong").await;
                            }
                        }
                    }
                    serenity::Error::Model(err) => {
                        let _ = ctx.say_error(format!("{}", err)).await;
                    }
                    _ => {
                        let _ = ctx.say_error("Something went wrong").await;
                    }
                }
            }
            Err(err) => {
                let _ = ctx.say_error("Something went wrong").await;
                tracing::warn!(
                    error.command_name = %ctx.command().qualified_name.as_str(),
                    error.message = %err,
                    "Internal error [handling {}]: {} ({:#?})",
                    ctx.command().name,
                    &err,
                    &err
                );
            }
        },
    }
}
