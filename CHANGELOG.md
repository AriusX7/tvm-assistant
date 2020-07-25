# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1-alpha] - 2020-07-25

### Commands

- Added ping command

### Dependencies

- Updated serenity to PR 905
- Updated sqlx to use master (v0.4-pre)
- Added vendored-openssl
- Updated others to use latest version

### Documentation

- Updated documentation to include info about precompiled binaries

### Misc

- Added Travis and Appveyor configuration to publish precompiled binaries
- Added database migrations

## [0.2.0] - 2020-07-20

Between 0.1.0 and 0.2.0, a lot of commands were fixed and new commands were added. Most of them are, unfortunately, not documented here.

### Commands

- Added `--all` flag to `votecount` and `players` command
- Added support server link to `info` command
- Fixed small bugs and typos with others commands

### Documentation

- Added documentation for previous/outdated versions
- Improve self-hosting documentation
- Added documentation for missing commands
- Updated documentation for some commands

### Database

- Added `players bigint[]` column to config table

## [0.1.0] - 2020-06-19

Released first version of Rust rewrite of TvM Assistant.
