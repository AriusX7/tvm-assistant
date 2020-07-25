-- Add migration script here
CREATE TABLE logging (
  guild_id bigint NOT NULL PRIMARY KEY,
  log_channel_id bigint,
  blacklist_channel_ids bigint [],
  whitelist_channel_ids bigint []
);
