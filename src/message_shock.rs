use std::time::Duration;

use log::{debug, info, trace};
use regex::Regex;
use serenity::{
    all::{Message, UserId},
    client::Context,
};

use crate::context;

pub(crate) async fn word_shock(ctx: Context, msg: Message) {
    let mut data = ctx.data.write().await;
    let config = data.get::<context::Config>().unwrap().clone();
    let shocker = data.get::<context::Shocker>().unwrap().clone();

    let split_sentence = Regex::new(r"(\b[^\s]+\b)").unwrap();

    let message_words: Vec<String> = split_sentence
        .captures_iter(&msg.content)
        .map(|x| x.get(0).unwrap().as_str().to_owned())
        .collect();

    trace!(
        "Message text: \"{}\" -> Split into: \"{}\"",
        msg.content,
        message_words.join(", ")
    );

    let mut do_shock = 'do_shock: {
        // If the message mentions any of the bot's operators, set do_shock to true.
        if config
            .discord_config
            .operator_ids
            .iter()
            .any(|x| msg.mentions_user_id(<u64 as Into<UserId>>::into(*x)))
        {
            trace!("Message mentions bot owner. Shock impending...");
            break 'do_shock true;
        }

        // If any of the words in the message are a trigger word, set do_shock to true.
        for word in message_words {
            let word_lowercase = word.to_lowercase();
            if config.trigger_words.iter().any(|x| *x == word_lowercase) {
                trace!("Caught trigger word. Shock impending...");
                break 'do_shock true;
            }
        }

        // If no predicates match...
        trace!("Message does not match shock parameters.");
        false
    };

    {
        let user_shock_cooldowns = data.get_mut::<context::UserShockCooldowns>().unwrap();

        if do_shock
            && !user_shock_cooldowns
                .entry(msg.author.id)
                .or_insert(ShockCooldown {
                    stopwatch: std::time::Instant::now(),
                    shock_count: 0,
                })
                .can_shock(
                    std::time::Duration::from_secs(config.cooldown_segment_duration as u64),
                    config.max_shocks_per_segment,
                )
        {
            // The number of seconds until the segment counter is reset.
            let seconds_until_reset = config.cooldown_segment_duration as u64
                - user_shock_cooldowns
                    .get(&msg.author.id)
                    .unwrap()
                    .stopwatch
                    .elapsed()
                    .as_secs();

            debug!(
            "User has exceeded shock limit for the current segment {}/{} ({} seconds remaining)",
            user_shock_cooldowns
                .get(&msg.author.id)
                .expect(
                    format!(
                        "Could not access user shock cooldown for {}.",
                        msg.author.name
                    )
                    .as_str()
                )
                .shock_count,
            config.max_shocks_per_segment,
            seconds_until_reset,
        );
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Wait {} seconds...", seconds_until_reset),
                )
                .await
                .unwrap();
            do_shock = false;
        }
    }

    if do_shock {
        let user_shock_cooldowns = data.get_mut::<context::UserShockCooldowns>().unwrap();

        let typing = msg.channel_id.start_typing(&ctx.http);

        info!("Shocking!");
        shocker.shock(40, Duration::from_secs(1)).await.unwrap();

        typing.stop();

        // Update shock cooldown by adding 1 to shock_count.
        let mut shock_cooldown = user_shock_cooldowns.get(&msg.author.id).unwrap().clone();
        shock_cooldown.shock_count += 1;
        user_shock_cooldowns.insert(msg.author.id, shock_cooldown);
    }
}

/// The number of shocks a user has dealt during the current span of time.
#[derive(Debug, Clone)]
pub(crate) struct ShockCooldown {
    /// The timestamp that the block started at.
    stopwatch: std::time::Instant,
    /// The number of times the user has dealt a shock during the last block.
    shock_count: u32,
}

impl ShockCooldown {
    /// Are there room for more shocks during the current segment? Returns true if the cooldown has room for the shock. Returns false if too many shocks have already been dealt.
    ///
    /// # Parameters
    /// * `segment_length` - The amount of time between segment resets.
    /// * `maximum_shocks` - The maximum number of shocks allowed before a segment reset.
    fn can_shock(&mut self, segment_length: std::time::Duration, maximum_shocks: u32) -> bool {
        // Reset the stopwatch and shock_count if the segment_length has been reached.
        if self.stopwatch.elapsed() >= segment_length {
            self.stopwatch = std::time::Instant::now();
            self.shock_count = 0;
        }

        // Check to see if the shock_count is below or equal to maximum. Return true if so, and increment shock_count.
        if self.shock_count < maximum_shocks {
            true
        } else {
            false
        }
    }
}
