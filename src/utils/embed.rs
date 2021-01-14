//! An alternative `Embed` builder. It allows for easy customisation and chaining.

use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp},
    utils::Colour,
};

#[derive(Debug, Clone)]
pub struct EmbedAuthor {
    pub icon_url: Option<String>,
    pub name: String,
    pub url: Option<String>,
}

#[allow(unused)]
impl EmbedAuthor {
    pub fn new<S: ToString>(name: S) -> Self {
        Self {
            icon_url: None,
            name: name.to_string(),
            url: None,
        }
    }

    pub fn icon_url<S: ToString>(mut self, icon_url: S) -> Self {
        self.icon_url = Some(icon_url.to_string());
        self
    }

    pub fn name<S: ToString>(mut self, name: S) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn url<S: ToString>(mut self, url: S) -> Self {
        self.url = Some(url.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct EmbedFooter {
    pub icon_url: Option<String>,
    pub text: String,
}

#[allow(unused)]
impl EmbedFooter {
    pub fn new<S: ToString>(text: S) -> Self {
        Self {
            icon_url: None,
            text: text.to_string(),
        }
    }

    pub fn icon_url<S: ToString>(mut self, icon_url: S) -> Self {
        self.icon_url = Some(icon_url.to_string());
        self
    }

    pub fn text<S: ToString>(mut self, text: S) -> Self {
        self.text = text.to_string();
        self
    }
}

#[derive(Debug, Clone)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

impl EmbedField {
    pub fn new<T: ToString, U: ToString>(name: T, value: U, inline: bool) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            inline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Embed {
    pub author: Option<EmbedAuthor>,
    pub colour: Option<Colour>,
    pub description: Option<String>,
    pub fields: Vec<EmbedField>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<String>,
    pub thumbnail: Option<String>,
    pub timestamp: Option<Timestamp>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub attachment: Option<String>,
}

#[allow(unused)]
impl Embed {
    pub fn new() -> Self {
        Self {
            author: None,
            colour: None,
            description: None,
            fields: Vec::new(),
            footer: None,
            image: None,
            thumbnail: None,
            timestamp: None,
            title: None,
            url: None,
            attachment: None,
        }
    }

    pub fn author(mut self, author: EmbedAuthor) -> Self {
        self.author = Some(author);
        self
    }

    pub fn colour<C: Into<Colour>>(mut self, colour: C) -> Self {
        self.colour = Some(colour.into());
        self
    }

    pub fn description<S: ToString>(mut self, description: S) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn field<T, U>(mut self, field: (T, U, bool)) -> Self
    where
        T: ToString,
        U: ToString,
    {
        self.fields.push(EmbedField::new(field.0, field.1, field.2));
        self
    }

    pub fn fields<T, U, It>(mut self, fields: It) -> Self
    where
        It: IntoIterator<Item = (T, U, bool)>,
        T: ToString,
        U: ToString,
    {
        let mut fields: Vec<EmbedField> = fields
            .into_iter()
            .map(|(n, v, b)| EmbedField::new(n, v, b))
            .collect();
        self.fields.append(&mut fields);
        self
    }

    pub fn footer(mut self, footer: EmbedFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn image<S: ToString>(mut self, image: S) -> Self {
        self.image = Some(image.to_string());
        self
    }

    pub fn thumbnail<S: ToString>(mut self, thumbnail: S) -> Self {
        self.thumbnail = Some(thumbnail.to_string());
        self
    }

    pub fn timestamp<T: Into<Timestamp>>(mut self, timestamp: T) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    pub fn title<S: ToString>(mut self, title: S) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn url<S: ToString>(mut self, url: S) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn get_create_embed(self) -> CreateEmbed {
        let mut embed = CreateEmbed::default();

        if let Some(item) = self.author {
            let mut author = CreateEmbedAuthor::default();

            author.name(item.name);

            if let Some(value) = item.icon_url {
                author.icon_url(value);
            }
            if let Some(value) = item.url {
                author.url(value);
            }

            embed.author(|f| {
                f.0 = author.0;

                f
            });
        }

        if let Some(value) = self.colour {
            embed.colour(value);
        }

        if let Some(value) = self.description {
            embed.description(value);
        }

        // Set fields.
        for field in self.fields {
            embed.field(field.name, field.value, field.inline);
        }

        if let Some(item) = self.footer {
            let mut footer = CreateEmbedFooter::default();

            footer.text(item.text);

            if let Some(value) = item.icon_url {
                footer.icon_url(value);
            }

            embed.footer(|f| {
                f.0 = footer.0;

                f
            });
        }

        if let Some(value) = self.image {
            embed.image(value);
        }

        if let Some(value) = self.thumbnail {
            embed.thumbnail(value);
        }

        if let Some(value) = self.timestamp {
            embed.timestamp(value);
        }

        if let Some(value) = self.title {
            embed.title(value);
        }

        if let Some(value) = self.url {
            embed.url(value);
        }

        if let Some(value) = self.attachment {
            embed.attachment(value);
        }

        embed
    }

    /// Sets field at position `index`, if it is within bounds.
    /// This function modifies the embed in place and also returns it
    /// to allow chaining.
    ///
    /// The function consumes`field`.
    pub fn set_field_at(mut self, index: usize, field: EmbedField) -> Self {
        if self.fields.len() - 1 > index {
            self.fields[index] = field;
        }

        self
    }
}
