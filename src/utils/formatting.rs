// Various functions to format text and files.

use serenity::http::AttachmentType;
use std::borrow::Cow;

pub(crate) struct PagifyOptions<'a> {
    delims: &'a [&'a str],
    escape_mass_mentions: bool,
    shorten_by: usize,
    page_length: usize,
}

impl<'a> PagifyOptions<'a> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[allow(unused)]
    pub(crate) fn delims(mut self, delims: &'a [&'a str]) -> Self {
        self.delims = delims;
        self
    }

    #[allow(unused)]
    pub(crate) fn escape_mass_mentions(mut self) -> Self {
        self.escape_mass_mentions = true;
        self
    }

    pub(crate) fn shorten_by(mut self, length: usize) -> Self {
        self.shorten_by = length;
        self
    }

    pub(crate) fn page_length(mut self, length: usize) -> Self {
        self.page_length = length;
        self
    }
}

impl<'a> Default for PagifyOptions<'a> {
    fn default() -> Self {
        Self {
            delims: &["\n"],
            escape_mass_mentions: true,
            shorten_by: 8,
            page_length: 2000,
        }
    }
}

/// Breaks a large chuck of text into smaller pages.
///
/// It can be fine-tuned by using appropriate `PagifyOptions`.
///
/// Source: https://github.com/Cog-Creators/Red-DiscordBot/blob/V3/develop/redbot/core/utils/chat_formatting.py#L212
pub(crate) fn pagify<S: ToString>(text: S, page_options: Option<PagifyOptions<'_>>) -> Vec<String> {
    let text = text.to_string();
    let mut in_text = text;
    let mut options = match page_options {
        Some(o) => o,
        None => PagifyOptions::default(),
    };

    let mut texts = Vec::new();

    options.page_length -= options.shorten_by;
    while in_text.len() > options.page_length {
        let mut this_page_len = options.page_length;

        if options.escape_mass_mentions {
            let sliced_text = match in_text.get(0..options.page_length) {
                Some(s) => s,
                None => continue,
            };
            this_page_len -=
                sliced_text.matches("@here").count() + sliced_text.matches("@everyone").count();
        }

        let closest_delim = match options
            .delims
            .iter()
            .filter_map(|&d| in_text[1..this_page_len].rfind(d))
            .max()
        {
            Some(d) => d,
            None => this_page_len,
        };

        let to_send = if options.escape_mass_mentions {
            escape(&in_text[..closest_delim], true)
        } else {
            in_text[..closest_delim].to_string()
        };
        if !to_send.is_empty() {
            texts.push(to_send);
        }
        in_text = in_text[closest_delim..].to_string();
    }

    if !in_text.trim().is_empty() {
        if options.escape_mass_mentions {
            texts.push(escape(in_text, true))
        } else {
            texts.push(in_text)
        }
    }

    texts
}

/// Returns text after escaping mass mentions (@everyone and @here).
///
/// A zero-width Unicode character (u200b) is added between `@` and `everyone` or `here`
/// to escape the mention.
pub(crate) fn escape<S: ToString>(text: S, mass_mentions: bool) -> String {
    let mut text = text.to_string();

    if mass_mentions {
        text = text.replace("@everyone", "@\u{200b}everyone");
        text = text.replace("@here", "@\u{200b}here");
    }

    text
}

/// Creates a `serenity::http::AttachmentType` from the given text.
pub(crate) fn text_to_file<'a, S: ToString, T: ToString>(
    text: S,
    file_name: T,
) -> AttachmentType<'a> {
    let data = Cow::from(text.to_string().into_bytes());
    AttachmentType::Bytes {
        data,
        filename: file_name.to_string(),
    }
}

use serenity::model::prelude::Message;

/// Returns string after capitalizing first letter and making all others lowercase.
/// Only works for strings with ASCII letters (a-z | A-Z).
pub(crate) fn capitalize(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut c = lower.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Returns the message content after cleaning up user mentions.
pub(crate) fn clean_user_mentions(message: &Message) -> String {
    let mut result = message.content.clone();

    for user in &message.mentions {
        result = result.replace(format!("<@{}>", user.id.0).as_str(), user.name.as_str());
        result = result.replace(format!("<@!{}>", user.id.0).as_str(), user.name.as_str());
    }

    result
}
