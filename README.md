<h1 align="center">
  <br>
  <a href="https://ariusx7.github.io/tvm-assistant/">
  <img src="https://i.imgur.com/v9WAfJi.jpg" alt="TvM Assistant">
  <br>
  </a>
  TvM Assistant
  <br>
</h1>

<h4 align="center">Makes hosting TvMs easier!</h4>

<p align="center">
  <a href="#introduction">Introduction</a>
  •
  <a href="#features">Features</a>
  •
  <a href="#documentation">Documentation</a>
  •
  <a href="#self-hosting">Self Hosting</a>
  •
  <a href="#credits">Credits</a>
</p>

## Introduction

TvM Assistant is a Discord bot with utility commands to make hosting and playing TvMs easier. You can invite it to your server by using [this link](https://discord.com/api/oauth2/authorize?client_id=680383600725590020&permissions=268494928&scope=bot). Inviting the bot will give it `Manage Channels`, `Manage Roles`, `Manage Messages`, `Add Reactions` and `Embed Links` permissions in addition to `Read` and `Send` messages perm.

TvM Assistant is written in [Rust](https://www.rust-lang.org), using the in-development `await` branch of [serenity](https://github.com/Lakelezz/serenity/tree/await).

## Features

- Setup roles and channel creation
- Management of sign-ups, sign-outs, spectators and replacements
- Day/night cycle management
- In-built logging to detect and ignore private channels
- Quick creation of player, mafia and spectator chats
- Vote counts and time since day/night started
- And more!

Suggest a feature by sending me a message on Discord (`Arius#5544`).

## Documentation

Detailed instructions on setting up the bot, commands, self-hosting, etc. are available [here](https://ariusx7.github.io/tvm-assistant/).

## Self Hosting

If you'd like to host the legacy [Python](https://www.python.org) version of the bot, check out [this page](https://ariusx7.github.io/tvm-assistant-red-cog/#self-hosting). Note that the Python version is no longer developed and may not work. However, it is comparatively easier to host. To host the latest version, written in [Rust](https://www.rust-lang.org/), continue reading.

### Prerequisites

Hosting TvM Assistant isn't very easy. To host the bot on your own, you'll need:

- A computer
- [Discord Bot Application](#discord-bot-application)
- [PostgreSQL](#postgresql)
- [Source code of this bot](#source-code)
- [Rust](#rust)
- [wkhtmltopdf](https://wkhtmltopdf.org)

You need to have some knowledge of command prompt ([Windows](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/windows-commands)) or terminal ([MacOS](https://support.apple.com/en-in/guide/terminal/welcome/mac), [other Unix-like](https://en.wikipedia.org/wiki/List_of_Unix_commands)) commands. You'll have to use the command prompt/terminal to host the bot.

No knowledge of Rust is required.

Brief instructions to install/create the above requirements given below. If you run into errors, you can contact me on Discord (`Arius#5544`), but please try using Google to fix the errors first, as you will be able find a solution for most of the errors you may run into.

### Installation

**Note 1:** Throughout this section, "terminal" will refer to both command prompt (Windows) and terminal (MacOS, Unix-like), unless otherwise stated.

**Note 2:** Some commands are prefixed with `$`. Remove the `$` before running the commands.

**Note 3:** The instructions assume that you're setting up a bot using a VPS hosting service, Raspberry Pi or a 24/7 on computer. You can set up the bot on a regular computer, but the bot will stop working as soon as you turn off the computer (or, to be precise, close the terminal).

#### Discord Bot Application

You can follow [this guide on discord.py](https://discordpy.readthedocs.io/en/latest/discord.html) to create a bot application. There are some `Python` specific instructions at the bottom of the page. You can safely ignore them. Make sure to note your bot application's token somewhere, you'll need it to set up the bot. Don't worry if you weren't able to note it, you can always check it again.

**Never share your token with anybody. Anyone with your bot's token can assume full control of your bot. If you accidentally shared the token, go back to your bot application's page and regenerate the token as soon as possible.**

#### PostgreSQL

You can download PostgreSQL [here](https://www.postgresql.org/download/). Choose an option that works for your operating system.

Firstly, you'll need to create PostgreSQL database and tables. Make sure you have installed PostgreSQL correctly and added it to your `PATH`. Instructions to do that can be found on Google. [DigitalOcean has a very informative guide for installing PostgreSQL on Ubuntu 18.04](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-postgresql-on-ubuntu-18-04).

To see if you have PostgreSQL installed, run this command in a terminal window:

```$ psql -V```

If the output is similar to `psql (PostgreSQL) 12.2` (version can be different, but make sure it's not a very old version), you've installed it correctly.

Now, you need to create a database. It can be done by using the following command:

```$ createdb database_name```

There are some best practices involved with creating databases. You can find those and ways to fix common issues with that command on [this page](https://www.postgresql.org/docs/12/tutorial-createdb.html).

After creating the database, you will need to access it. It is done by using:

```$ psql -d database_name```

For more information about this command, [see this page](https://www.postgresql.org/docs/12/tutorial-accessdb.html).

Inside the database, you'll need to create 3 tables. Use the following commands to create them:

*Table to store TvM configuration and states data:*

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

*Table to store logging configuration:*

```sql
CREATE TABLE logging (
  guild_id bigint NOT NULL PRIMARY KEY,
  log_channel_id bigint,
  blacklist_channel_ids bigint [],
  whitelist_channel_ids bigint []
);
```

*Table to store custom prefixes:*

```sql
CREATE TABLE prefixes (
    guild_id bigint NOT NULL PRIMARY KEY,
    prefix text
);
```

Make sure you use these commands **inside** `psql` .

#### Source Code

You can download this bot's source code in one of the following two ways:

- You can download zip folder of this repository by [clicking here](https://codeload.github.com/AriusX7/tvm-assistant/zip/master).
- You can use [Git](https://git-scm.com) to clone this repository locally by running `$ git clone https://github.com/AriusX7/tvm-assistant.git` command in terminal.

After downloading the repository/folder and unzipping it, `cd` into it. You'll need to edit one file before building the bot. You can do that by opening the folder with your choice of text editor, like [Visual Studio Code](https://code.visualstudio.com/download), [Atom](https://atom.io), and [Sublime Text](https://www.sublimetext.com), if you're hosting the bot on a computer. If you're using a VPS, you'll probably need to use the terminal. Using the terminal, first run this command:

```$ mv .env.example .env```

Using the above command will rename `.env.example` file to `.env`. Next, use `nano`, `vim` or any other terminal editors to edit the `.env` file. If you're using a text editor on a computer, you'll have to do it yourself.

Put your bot application's token, which you created in [this section](#discord-bot-application), after `DISCORD_TOKEN=`. Don't leave any spaces or wrap it up in quotes. You can leave the `RUST_LOG` field as it is, or change it if you want to customize the logging. Lastly, add your Postgres database url. The URL has the following fields:

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

#### Rust

You can download Rust [here](https://www.rust-lang.org/tools/install). Choose an option that works for your operating system.

To check if you installed Rust correctly, run the following command:

```$ rustc -V```

If you see output similar to `rustc 1.44.0 (49cae5576 2020-06-01)` (version number, date and tag can differ, but make sure it's not a very old version), you've installed it correctly.

#### wkhtmltopdf

You can download wkhtmltopdf [here](https://wkhtmltopdf.org/downloads.html). Choose an option that works for your operating system.

Download one of the precompiled libraries instead of building it from source or using a package manager to install it. The precompiled libraries are patched with `QT`, which is a prerequisite for `wkhtmltopdf`. If you insist on building it from the source or using a package manager, you'll have to set up [QT](https://www.qt.io) yourself.

On Linux systems, you'll have to unzip the downloaded package, download missing dependencies and create symbolic links. You can do that by using these commands:

```shell
# These commands are for Ubuntu 18.04 x86_64 machines. See caveats after the command for more info.
sudo wget https://builds.wkhtmltopdf.org/0.12.6-1/wkhtmltox_0.12.6-1~bionic_amd64.deb
sudo dpkg -i wkhtmltox_0.12.6-1~bionic_amd64.deb
sudo apt-get install -f
sudo ln -s /usr/local/bin/wkhtmltopdf /usr/bin
sudo ln -s /usr/local/bin/wkhtmltoimage /usr/bin
```

**Caveats**
In the first command, the link after `wget` is the link to the precompiled binary on the downloads page of `wkhtmltopdf`. Make sure you copy the link appropriate for your operating system and architecture. `amd64` and `x86_64` refer to the same architecture.

In the second command, you may need to change `0.12.6-1~bionic_amd64.deb`. Use the URL you copied to edit it.

To check if you installed `wkhtmltopdf` correctly, run the following command:

```$ wkhtmltopdf --version```

If the output is similar to `wkhtmltopdf 0.12.6 (with patched qt)`, then you've installed it correctly. The version number can differ, but `(with patched qt)` should appear.

### Building The Bot

Now that you have all the prerequisities, you'll need to build the bot. Building the bot takes a long time, sometimes even more than 20 minutes, depending on the operating system and the RAM available. Generally, you'll want to have more than 750 MB RAM available to build the bot.

```sh
cargo build --release
```

You may run into some dependency errors when using this command, particularly with the `sys` crate. The errors are easy to resolve -- most of the times you are just missing a required library. The error will tell you what you're missing. Simply Google how to install it for your operating system and re-run the command. You don't need to worry about having to build it from stratch again, `cargo` will resume building from the point where it got an error.

If you get any critical errors, open an issue or contact me directly on Discord (`Arius#5544`).

Once the above command runs without any errors, you'll simply have to use

```sh
cargo run --release
```

to run the bot. Whenever you shutdown your bot, you'll have to re-run it using the above command. Running it will be almost instantaneous.

## Credits

- [Town of Salem](https://www.blankmediagames.com): Bot logo and README wallpaper
- [serenity](https://github.com/serenity-rs/serenity): The library used to create TvM Assistant
- And all the very helpful people on [serenity](https://discord.gg/WBdGJCc) and [The Rust Programming Language](https://discord.gg/rust-lang) Discord servers.
