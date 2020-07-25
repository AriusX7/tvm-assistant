<h1 align="center"><a href=".">Main Index</a></h1>

<p align="center">
  <a href="#prerequisites">Prerequisites</a>
  •
  <a href="#installation">Installation</a>
  •
  <a href="#running-the-bot">Running The Bot</a>
  •
  <a href="#updating-the-bot">Updating The Bot</a>
</p>

If you'd like to host the legacy [Python](https://www.python.org) version of the bot, check out [this page](https://ariusx7.github.io/tvm-assistant-red-cog/#self-hosting). Note that the Python version is no longer developed and may not work. To host the latest version, written in [Rust](https://www.rust-lang.org/), continue reading.

To host previous Rust versions, [click here](outdated/index).

## Prerequisites

There are two methods of hosting the latest versions of TvM Assistant.

- Using a precompiled binary (recommended)
- Building the bot from source

Both of the methods have some common requirements. In the below list, items without a marking (<sub><sup>††</sup></sub>) are required for both methods.

- A computer
- [Discord Bot Application](#discord-bot-application)
- [PostgreSQL](#postgresql)
- [Precompiled binary of this bot](#precompiled-binary)<sub><sup>†</sup></sub>
- [Source code of this bot](#source-code)<sub><sup>††</sup></sub>
- [Rust](#rust)<sub><sup>††</sup></sub>
- [wkhtmltopdf](https://wkhtmltopdf.org)<sub><sup>†††</sup></sub>

You need to have some knowledge of command prompt ([Windows](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/windows-commands)) or terminal ([MacOS](https://support.apple.com/en-in/guide/terminal/welcome/mac), [other Unix-like](https://en.wikipedia.org/wiki/List_of_Unix_commands)) commands. You'll have to use the command prompt/terminal to host the bot.

No knowledge of Rust is required.

Brief instructions to install/create the above requirements given below. If you run into errors, you can contact me on Discord (`Arius#5544`), but please try using Google to fix the errors first, as you will be able find a solution for most of the errors you may run into.

<sub><sup>† Only required for method 1 (recommended)</sup></sub>  
<sub><sup>†† Only required for method 2</sup></sub>  
<sub><sup>††† Only required if you want the `format` command to work</sup></sub>  

## Installation

**Note 1:** Throughout this section, "terminal" will refer to both command prompt (Windows) and terminal (MacOS, Unix-like), unless otherwise stated.

**Note 2:** Some commands are prefixed with `$`. Remove the `$` before running the commands.

**Note 3:** The instructions assume that you're setting up a bot using a VPS hosting service, Raspberry Pi or a 24/7 on computer. You can set up the bot on a regular computer, but the bot will stop working as soon as you turn off the computer (or, to be precise, close the terminal).

### Discord Bot Application

You can follow [this guide on discord.py](https://discordpy.readthedocs.io/en/latest/discord.html) to create a bot application. There are some `Python` specific instructions at the bottom of the page. You can safely ignore them. Make sure to note your bot application's token somewhere, you'll need it to set up the bot. Don't worry if you weren't able to note it, you can always check it again.

**Never share your token with anybody. Anyone with your bot's token can assume full control of your bot. If you accidentally shared the token, go back to your bot application's page and regenerate the token as soon as possible.**

### PostgreSQL

You can download PostgreSQL [here](https://www.postgresql.org/download/). Choose an option that works for your operating system.

Make sure you have installed PostgreSQL correctly and added it to your `PATH`. Instructions to do that can be found on Google. [DigitalOcean has a very informative guide for installing PostgreSQL on Ubuntu 18.04](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-postgresql-on-ubuntu-18-04). Please don't create a user or database yet, we'll do that later.

To see if you have PostgreSQL installed, run this command in a terminal window:

```$ psql -V```

If the output is similar to `psql (PostgreSQL) 12.2` (version can be different, but make sure it's not a very old version), you've installed it correctly.

We'll now create a new user. Use the following command:

```sh
createuser user_name --superuser --pwprompt
```

Replace `user_name` with name of your choice. Please keept it simply and easy to type.

### Precompiled Binary

You can download precompiled binaries of the bot from [this page](https://github.com/AriusX7/tvm-assistant/releases). Go the version you wish to host and click on the "Assets" dropdown. You'll see a list of files for different operating systems and architectures.

If you're hosting on a Mac or Windows computer, you can download the precompiled library by simply clicking on the link and unpacking the `tar.gz`/`zip` file.

On Linux, you'll need to use the following command:

```sh
sudo wget link_to_binary
tar xzf name_of_tar_file
```

Example commands:

```sh
sudo wget https://github.com/AriusX7/tvm-assistant/releases/download/v0.2.1-alpha/tvm-assistant-v0.2.1-alpha-x86_64-unknown-linux-gnu.tar.gz
tar xzf tvm-assistant-v0.2.1-alpha-x86_64-unknown-linux-gnu.tar.gz
```

It will unpack the tar file in the directory where you use the command. You can safely remove the tar file now.

```sh
rm -rf name_of_tar_file
```

### Source Code

You can download this bot's source code in one of the following two ways:

- You can download zip folder of this repository by [clicking here](https://codeload.github.com/AriusX7/tvm-assistant/zip/master).
- You can use [Git](https://git-scm.com) to clone this repository locally by running `$ git clone https://github.com/AriusX7/tvm-assistant.git` command in terminal.

### Setting Up Environment

Once you download and unpack a precompiled binary of the bot, or download the source code, you'll see a file named `.env.example` in the root directory. You'll have to create a new file based on it and fill in some values.

You can do that by opening the folder with your choice of text editor, like [Visual Studio Code](https://code.visualstudio.com/download), [Atom](https://atom.io), and [Sublime Text](https://www.sublimetext.com), if you're hosting the bot on a computer. If you're using a VPS, you'll probably need to use the terminal. Using the terminal, first run this command:

```sh
mv .env.example .env
```

It will rename `.env.example` file to `.env`. Next, use `nano`, `vim` or any other terminal editors to edit the `.env` file. If you're using a text editor on a computer, you'll have to do edit the file name yourself.

Put your bot application's token, which you created in [this section](#discord-bot-application), after `DISCORD_TOKEN=`. Don't leave any spaces or wrap it up in quotes. You can leave the `RUST_LOG` field as it is, or change it if you want to customize the logging. Lastly, add your Postgres database url. The URL has the following fields:

```py
username # your user name
password # "your password"
database_name # database name
host # host_name, it is usually "localhost"
port # the port number, it is usually "5432"
```

In the [PostgreSQL section](#postgresql), we downloaded PostreSQL but did not create any database. In the above URL, enter database name which you'll use for TvM Assistant. If your user name is `arius`, password is `12345678`, host is `localhost`, port is `5432` and database name is `tvm_assist`, your URL should look like this:

```postgres://arius:12345678@localhost:5432/tvm_assist```

And the `.env` file should look like this:

```text
DISCORD_TOKEN=YOUR_BOT_TOKEN
RUST_LOG=info
DATABASE_URL=postgres://arius:12345678@localhost:5432/tvm_assist
```

Now, we'll create the database and all the necessary tables.

#### Using `sqlx-cli`

The recommended way to do that is to use `sqlx-cli`. It is a command line tool that allows us to set up databases with very few commands.

**Note:** Currently, `sqlx-cli` is in alpha stage and does not have precompiled libraries. To install it, you'll need `Rust`. Please [install Rust](#rust) first and then come back to this section to install `sqlx-cli`. If you'd rather set up the database yourself, head over to [this section](#using-psql).

After installing Rust, use the following command:

```cargo install sqlx-cli --git https://github.com/launchbadge/sqlx.git```

The installation can take a long time to finish. Once it finishes installing, run the following commands:

```sh
sqlx database create
sqlx migrate run
```

It will automatically create the database and the tables for you.

#### Using `psql`

First, you'll have to create a database. Make sure the database_name is the same as the name you used in the `.env` file.

```$ createdb database_name```

There are some best practices involved with creating databases. You can find those and ways to fix common issues with that command on [this page](https://www.postgresql.org/docs/12/tutorial-createdb.html).

After creating the database, you will need to access it. It is done by using:

```$ psql database_name```

For more information about this command, [see this page](https://www.postgresql.org/docs/12/tutorial-accessdb.html).

Now, make sure the owner of the database is the user we put in `.env` file by running this command:

```sql
ALTER DATABASE database_name OWNER to user_name;
```

`user_name` should be same as the user name you entered in the env file.

Now, you'll need to create 3 tables. Use the following commands to create them:

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
  cycle jsonb,
  players bigint []
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

Make sure you use these commands **inside** `psql`.

---

If you used method 1 (downloaded precompiled binary of the bot), all you need to do now is download [wkhtmltopdf](#wkhtmltopdf) if you need `format` command and then [run the bot](#running-the-bot).

If you used method 2 (downloaded source code), you'll need to install Rust (if you haven't already), [wkhtmltopdf](#wkhtmltopdf) if you need `format` command and finally [build and run the bot](#running-the-bot).

### Rust

You can download Rust [here](https://www.rust-lang.org/tools/install). Choose an option that works for your operating system.

To check if you installed Rust correctly, run the following command:

```$ rustc -V```

If you see output similar to `rustc 1.44.0 (49cae5576 2020-06-01)` (version number, date and tag can differ, but make sure it's not a very old version), you've installed it correctly.

### wkhtmltopdf

You can download wkhtmltopdf [here](https://wkhtmltopdf.org/downloads.html). Choose an option that works for your operating system.

Download one of the precompiled libraries instead of building it from source or using a package manager to install it. The precompiled libraries are patched with `QT`, which is a prerequisite for `wkhtmltopdf`. If you insist on building it from the source or using a package manager, you'll have to set up [QT](https://www.qt.io) yourself.

On Linux systems, you'll have to unzip the downloaded package, download missing dependencies and create symbolic links. You can do that by using these commands:

```shell
# These commands are for Ubuntu 18.04 x86_64 machines. See caveats after the commands for more info.

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

## Running The Bot

Now that you have all the prerequisities, you'll need to (build and) run the bot. If you downloaded a precompiled binary (method 1), read the  [running precompiled binary](#running-precompiled-binary) section. Otherwise, read the [building and running from source](#building-and-running-from-source) section.

### Running Precompiled Binary

`cd` inside the bot's root directory (the directory where the contents of the tar/zip file were unpacked).

If you're on Linux or Mac, use the following command to run the bot:

```sh
./tvm-assistant
```

On Windows, run this command:

```sh
tvm-assistant.exe
```

And that's all! The bot will continue running as long as the terminal is open. To keep the bot running even when you close the terminal, you'll have to use an auto-restart facility like `systemd` on Linux. Instructions to set that up are not available yet.

To update the bot, you'll simply have to download a new binary and run the above command again.

### Building and Running From Source

Building the bot takes a long time, sometimes even more than 20 minutes, depending on the operating system and the RAM available. Generally, you'll want to have more than 750 MB RAM available to build the bot.

Inside the bot's root directory, run this command:

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

After each update, you'll have to rebuild and run the bot.

## Updating The Bot

To update the bot, you'll have to download a new precompiled binary from the releases page or the source code.

After downloading it, you'll have to run sqlx migrations. If you have `sqlx-cli` downloaded, you simply need to run the following command:

```sqlx migrate run```

If not, you'll find manual migration details on the [Releases](https://github.com/AriusX7/tvm-assistant/releases) page.

After migrating the database, you'll have to [run the bot](#running-the-bot).
