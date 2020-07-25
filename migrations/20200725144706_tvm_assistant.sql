-- Add migration script here

-- config table
CREATE TABLE config (
  guild_id bigint NOT NULL PRIMARY KEY,
  host_role_id bigint,
  player_role_id bigint,
  spec_role_id bigint,
  repl_role_id bigint,
  dead_role_id bigint,
  na_channel_id bigint,
  signups_channel_id bigint,
  can_change_na bool,
  tvmset_lock bool,
  signups_on bool,
  total_players smallint,
  total_signups smallint,
  na_submitted bigint [],
  cycle jsonb,
  players bigint []
);

-- logging table
CREATE TABLE logging (
  guild_id bigint NOT NULL PRIMARY KEY,
  log_channel_id bigint,
  blacklist_channel_ids bigint [],
  whitelist_channel_ids bigint []
);

-- prefixes table
CREATE TABLE prefixes (
    guild_id bigint NOT NULL PRIMARY KEY,
    prefix text
);
