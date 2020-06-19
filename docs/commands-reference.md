<h1 align="center"><a href=".">Main Index</a></h1>

<p align="center">
  <a href="#general">General</a>
  •
  <a href="#roles">Roles</a>
  •
  <a href="#channels">Channels</a>
  •
  <a href="#tvm-specific-settings">TvM Specific Commands</a>
  •
  <a href="#cycle-commands">Cycle Commands</a>
  •
  <a href="#misc">Misc</a>
</p>

**Note:** The stuff written inside `<>` and `[]` are called parameters. `<>` means required parameter, `[]` means optional. For example, if there is `<number>` written, simply enter a number, like `12`, without the `<>`. More on types of parameters [here](parameters).

## General

These commands can be used by everyone, unless otherwise stated.

### `-in [ignored]`

Sign-up for the TvM. Automatically assigns the Player role and removes Spec and Replacement roles, if necessary. You can type anything you want after `-in`.

*Hosts, don't use this command.*

### `-spec [ignored]`

Sign-out of the TvM or spectate it. Automatically assigns the Spectator role and removes Player and Replacement roles, if necessary. You can type anything you want after `-spec`. Aliases: `-out`, `-spectator`

*Hosts, don't use this command.*

### `-repl [ignored]`

Sign-up as a replacement. Automatically assigns the Replacement role and removes Player and Spectator roles, if necessary. You can type anything you want after `-repl`.

*Hosts, don't use this command.*

### `-votecount [channel]`

Counts votes! The bot can automatically detect voting channels. However, it may not be able to detect the correct channel in some cases. Please specify the vote channel in such cases! Alias: `-vc [channel]`

### `-timesince`

Tells time elapsed since day/night channel was opened. The bot can automatically detect if it's day or night.

### `-nightaction <action>`

Your night action. It can only be used in your own private channel. `action` can be any text. You may not be able to update your night action if host has disabled that setting. Alias: `-na <action>`

*Can only be used by players.*

## Roles

These commands require administrator permission or the host role.

### `-tvm hostrole <role>`

Sets `role` as the host role.

*The bot asks for confirmation before making changes.*

### `-tvm playerrole <role>`

Sets `role` as the player role.

*The bot asks for confirmation before making changes.*

### `-tvm specrole <role>`

Sets `role` as the spectator role.

*The bot asks for confirmation before making changes.*

### `-tvm deadrole <role>`

Sets `role` as the dead player role.

*The bot asks for confirmation before making changes.*

### `-tvm replrole <role>`

Sets `role` as the replacement role.

*The bot asks for confirmation before making changes.*

### `-tvm setroles`

Creates all the 5 roles and sets them as appropriate automatically. Also adds the host role to the person using command.

## Channels

These commands require administrator permission or the host role.

### `-tvm nachannel <channel>`

Sets `channel` as the channel where night actions are relayed.

### `-tvm signup <channel>`

Sets `channel` as the sign-ups channel.

*The bot asks for confirmation before making changes.*

### `-tvm hostchannel`

Creates a channel for hosts. **The bot can see this channel.**

### `-tvm setchannels`

Creates `signups` and `nachannel` channels and sets them as appropriate automatically.

### `-playerchats [category_name]`

Sets up private channels for all players. It also sets up individual channels for players who are mafia. Please do not remove the mafia channels. Anyone can view the list of all server channels by using a custom Discord client. You can also specify a category name for the channels. Alias: `-pc [category_name]`

### `-mafiachat <mafia1> [mafia2 [mafia3..]]`

Sets up mafia chat for users specified. Example: `-mafiachat Arius#5544 Ligi @Siris#4421`. Alias: `-mafchat <mafia1> [mafia2 [mafia3..]]`

### `-specchat [channel]`

Creates spectator chat or fix permissions of an existing channel.

## TvM Specific Settings

These commands require administrator permission or the host role.

### `-tvm changena`

Toggle if user can change their night action once submitted. Defaults to `True`.

### `-tvm total <number>`

Sets `number` as the total number of players that can sign-up for the game. Defaults to `12`.

### `-tvm signopen`

Allows people to sign-up. On by default.

### `-tvm signclose`

Closes sign-ups.

### `-tvm lock`

Locks these, role and channel settings (commands that begin with `tvm`). Useful once the configuration is done so you don't accidentally mess things up mid-game.

### `-tvm unlock`

Unlocks TvM settings.

### `-tvm settings`

Shows TvM settings. Alias: `-tvm show`

## Cycle Commands

These commands require administrator permission or the host role.

### `-cycle [number]`

Creates a `Cycle` category with three channels: day, night and voting. Night channel is not visible to anyone except hosts and the bot. The bot maintains a cycle count (which starts at 0). However, the count may be broken in some cases. You can fix the count by specifying the cycle number when using the command. Simply using the `-cycle` command is recommended unless things are broken. Make sure to only use the command once per cycle.

Make sure you have the day opening text ready before using the command. Day and vote channels will be visible to users as soon as they are created.

*The bot asks for confirmation before making changes.*

### `-night`

Close the day and vote channels and open the night channel. Make sure you post the night starting text, minus the players ping, before using this command. Bot mentions the `Player` role once the channel is opened.

The bot may be unable to identify the correct day/night channels in some cases. In such cases, please open and close the channels yourself.

*The bot asks for confirmation before making changes.*

## Logging

These commands require administrator permission or the host role.

### `-log channel [channel]`

Sets or create sthe logging channel. All message edits and deletes from **public** channels will be logged in this channel.

If you just use `-log channel`, the bot will create a new channel called `log`. You can also set a channel you've already created as the log channel. Simply mention the channel in the command like this: `-log channel #channel-name`.

### `-log whitelist <channel>`

Whitelists `channel`. The messages in it will be logged irrespective of its permissions (as long as the bot can view this channel).

### `-log blacklist <channel>`

Blacklists `channel`. The messages in it will not be logged irrespective of its permissions. Whitelisting takes precedence over blacklisting. If a channel is in whitelist, the messages WILL BE LOGGED even if you blacklist it.

### `-log rwhitelist <channel>`

Removes `channel` from the whitelist.

### `-log rblacklist <channel>`

Removes `channel` from the blacklist.

### `-log settings`

Displays log settings.

## Misc

These commands require administrator permission or the host role, unless otherwise stated.

### `-rand <roles>`

Randomly assigns a role from the pool to a person with the `Player` role. The command should follow the pattern used in this example: `-rand role1, role2, role3, ...`. Number of players should be equal to number of roles. You can duplicate roles.

### `-players`

Lists all members with `Player` role.

*Can be used by everyone.*

### `-replacements`

Lists all members with `Replacement` role.

*Can be used by everyone.*

### `-kill <user>`

Kills a player by automatically removing player role and adding the dead player role.

### `-synctotal`

Sometimes the count of signups kept by the bot may not be able to the number of users who have actually signed up. Use this command to bring them into sync.

### `-started <cycle_number>`

Sometimes the bot can't determine if the game started or not. Use this to fix the issue. `cycle_number` is the number of cycle currently on.
