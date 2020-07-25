-- Add migration script here
CREATE TABLE prefixes (
    guild_id bigint NOT NULL PRIMARY KEY,
    prefix text
);
