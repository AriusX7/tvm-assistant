<h1 align="center"><a href=".">Main Index</a></h1>

If you'd like to host the legacy [Python](https://www.python.org) version of the bot, check out [this page](https://ariusx7.github.io/tvm-assistant-red-cog/#self-hosting). Note that the Python version is no longer developed and may not work. To host the latest version, written in [Rust](https://www.rust-lang.org/), continue reading.

To host the bot on your own, you'll need:

- A computer and basic command prompt ([Windows](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/windows-commands))/terminal ([MacOS](https://support.apple.com/en-in/guide/terminal/welcome/mac), [other Unix-like](https://en.wikipedia.org/wiki/List_of_Unix_commands)) knowledge
- [Rust](https://www.rust-lang.org/tools/install)
- [PostgreSQL](https://www.postgresql.org/download/)
- [Discord Bot Application](https://discord.com/developers/applications)
- A VPS hosting service, Raspberry Pi or a 24/7 on computer to keep the bot always online

**Note 1:** Throughout this section, "terminal" will refer to both command prompt (Windows) and terminal (MacOS, Unix-like), unless otherwise stated.

**Note 2:** Some commands are prefixed with `$`. Remove the `$` before running the commands.

You can follow [this guide on discord.py](https://discordpy.readthedocs.io/en/latest/discord.html) to create a bot application. You can ignore any `Python` specific instructions.

Once you have all the prerequisites, you'll need to follow these steps:

Firstly, you'd need to create PostgreSQL database and tables. Make sure you have installed PostgreSQL correctly and added it to your `PATH`. Instructions to do that can be found on Google. [DigitalOcean has a very informative guide for installing PostgreSQL on Ubuntu 18.04](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-postgresql-on-ubuntu-18-04).

To see if you have PostgreSQL, run this command in terminal:

```$ psql -V```

If the output is similar to `psql (PostgreSQL) 12.2` (version can be different, but make sure it's not a very old version), you've installed it correctly.

Now, you need to create a database. It can be done by using the following command:

```$ createdb database_name```

There are some best practices involved with create databases. You can find those and ways to fix common issues with that command on [this page](https://www.postgresql.org/docs/12/tutorial-createdb.html).

After creating the database, you will need to access it. It is done by using:

```$ psql -d database_name```

For more information about this command, [see this](https://www.postgresql.org/docs/12/tutorial-accessdb.html).

Inside the database, you'll need to create 3 tables. Use the following commands to create them:

Table to store TvM configuration and states data:

```sql
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
  cycle jsonb
);
```

Table to store logging configuration:

```sql
CREATE TABLE logging (
  guild_id bigint NOT NULL PRIMARY KEY,
  log_channel_id bigint,
  blacklist_channel_ids bigint [],
  whitelist_channel_ids bigint []
);
```

Table to store custom prefixes:

```sql
CREATE TABLE prefixes (
    guild_id bigint NOT NULL PRIMARY KEY,
    prefix text
);
```

Make sure you use these commands **inside** `psql` .

Next, you'll need to download the source code of this bot. It can be done in one of the following two ways:

- You can download zip folder of this repository by [clicking here](https://codeload.github.com/AriusX7/tvm-assistant/zip/master).
- You can use [Git](https://git-scm.com) to clone this repository locally by running `$ git clone https://github.com/AriusX7/tvm-assistant.git` command in terminal.

After downloading the repository/folder and unzipping it, `cd` into it. `cd` is a terminal command to change the current directory/folder. You'll need to edit one file before building the bot. You can do that by opening the folder with your choice of text editor, like [Visual Studio Code](https://code.visualstudio.com/download), [Atom](https://atom.io), and [Sublime Text](https://www.sublimetext.com), if you're hosting the bot on a computer. If you're using a VPS, you'll probably need to use the terminal. Using the terminal, first run this command:

```$ mv .env.example .env```

Next, use `nano`, `vim` or any other terminal editors to edit the `.env` file. Put your bot's token, which you can obtain [here](https://discord.com/developers/applications), after `DISCORD_TOKEN=`. Don't leave any spaces or wrap it up in quotes. You can leave the `RUST_LOG` field as it is, or change it if you want to customize the logging. Lastly, you'd need to enter your Postgres database url. The URL has the following fields:

```py
username # your user name
password # "your password"
database_name # database name
host # host_name, it is usually "localhost"
port # the port number, it is usually "5432"
```

Instructions to find these values can be easily found on Google. Once you have all the values, structure the URL as follows:

```postgres://username:password@host:port/database_name```

Example URL: `postgres://arius:12345678@localhost:5432/tvm_assist`

Now, you'll need to build the bot. For that, you'll need Rust. To check if you correctly installed Rust, run the following command:

```$ rustc -V```

If you see output similar to `rustc 1.44.0 (49cae5576 2020-06-01)` (version number, date and tag can differ, but make sure it's not a very old version), you've installed it correctly.
Now, you'll need to run the following command:

```$ cargo build --release```

This process will take a long time to finish. But once it is done, you'll simply have to use

```$ cargo run --release```

to run the bot. Whenever you shutdown your bot, you'll have to re-run it using the above command. Running it will be almost instantaneous.
