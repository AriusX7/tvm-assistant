-- Add migration script here
ALTER TABLE config ADD COLUMN notify_cooldown int NOT NULL DEFAULT 6;

CREATE TABLE cooldown(
    guild_id bigint NOT NULL PRIMARY KEY,
    cmd text NOT NULL,
    last_used timestamptz,
    CONSTRAINT unq_guild_cmd UNIQUE(guild_id, cmd)
);
