//! Various functions to format text and files.

use comrak::{
    markdown_to_html, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions,
    ComrakRenderOptions,
};
use serenity::{http::AttachmentType, model::prelude::Message};
use std::{fmt::Write as FmtWrite, fs, io::Write, path::Path};
use tokio::process::Command;
use tracing::{error, instrument};

static CSS: &str = include_str!("../../style.css");

/// Returns string after capitalizing first letter and making all others lowercase.
/// Only works for strings with ASCII letters (a-z | A-Z).
pub(crate) fn capitalize(s: &str) -> String {
    let mut capitalized = String::new();
    for word in s.split_whitespace() {
        for (i, c) in word.chars().enumerate() {
            if i == 0 {
                let _ = write!(capitalized, "{}", c.to_ascii_uppercase());
            } else {
                let _ = write!(capitalized, "{}", c.to_ascii_lowercase());
            }
        }
        let _ = write!(capitalized, " ");
    }

    capitalized.trim().to_string()
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

/// Takes `CommonMark` Markdown text as input and returns customised PDF and JPEG files.
///
/// This uses the `wkhtmltopdf` and `wkhtmltoimage` command-line tools.
/// The input Markdown is converted to HTML, which is then sanitized. A custom CSS (`style.css`)
/// is added to the HTML. The HTML is then used to create a `PDF` and a `JPEG` image.
///
/// Three files are created as side-effects:
///    * `foo.html`
///    * `out.pdf`
///    * `out.jpeg`
///
/// They must be handled by the calling function. They cannot be deleted before
/// they are sent as a message.
#[instrument]
pub(crate) async fn markdown_to_files<'a>(
    text: &str,
) -> (Option<AttachmentType<'a>>, Option<AttachmentType<'a>>) {
    let html = markdown_to_html(
        text,
        &ComrakOptions {
            extension: ComrakExtensionOptions {
                table: true,
                strikethrough: true,
                superscript: true,
                autolink: true,
                ..Default::default()
            },
            parse: ComrakParseOptions::default(),
            render: ComrakRenderOptions {
                github_pre_lang: true,
                ..Default::default()
            },
        },
    );

    let mut file = match fs::File::create("foo.html") {
        Ok(f) => f,
        Err(_) => {
            error!("Error creating `foo.html`.");
            return (None, None);
        }
    };

    let _ = write!(file, "<style>{}</style>{}", CSS, ammonia::clean(&html));

    let image_child = Command::new("wkhtmltoimage")
        // Max quality
        .args(&["--quality", "100"])
        // No output on stdout
        .arg("--quiet")
        // Input html file
        .arg("foo.html")
        // Output jpeg
        .arg("out.jpeg")
        .spawn();

    let pdf_child = Command::new("wkhtmltopdf")
        // No margins
        .args(&["-L", "0", "-R", "0", "-T", "0", "-B", "0"])
        // No output on stdout
        .arg("--quiet")
        // Input html file
        .arg("foo.html")
        // Output pdf
        .arg("out.pdf")
        .spawn();

    let image = match image_child {
        Ok(mut c) => c
            .wait()
            .await
            .ok()
            .map(|_| AttachmentType::Path(&Path::new("out.jpeg"))),
        Err(_) => None,
    };

    let pdf = match pdf_child {
        Ok(mut c) => c
            .wait()
            .await
            .ok()
            .map(|_| AttachmentType::Path(&Path::new("out.pdf"))),
        Err(_) => None,
    };

    (pdf, image)
}
