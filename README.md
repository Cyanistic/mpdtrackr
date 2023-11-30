<div align="center">

**A CLI [MPD](https://www.musicpd.org/) listening time tracker with versatile stats.**

[Features](#features) •
[Getting Started](#getting-started) •
[Usage](#usage)

</div>

## Features

- Keeps track of which songs are playing, how long they have been playing each day, and maintains historical playtime data to view trends over time
- Displays listening statistics directly in the terminal for easy access and analysis
  - Allows users to group stats by time periods (day, week, month, year, all-time) and different fields (artist, album, title, genre)
  - Permits sorting of statistics based on multiple criteria for in-depth analysis
- Facilitates hassle-free transfer of stats across devices using an SQLite database, compatible with tools like [syncthing](https://syncthing.net/)

## Getting Started

Ensure that you've installed the latest release from the [releases](https://github.com/Cyanistic/mpdtrackr/releases/latest) page.

Using mpdtrackr is as simple as running `mpdtrackr run` after starting up your MPD server.

In order for mpdtrackr to properly keep track of songs, your music files should have the proper title, artist, genre, etc. tags. Otherwise the application has to attempt to parse the title and artist from the file name, which can be inaccurate, and will not have any data for the album or genre.

To make sure that you never forget to start up the mpdtrackr daemon before listening to music, you probably want to make sure it runs on startup. There is a minimal `mpdtrackr.service` file provided to let you do that, provided that you use systemd. Otherwise you probably already know how to make a command run on startup using your window-manager configuration or something of the sort.

If you want to transfer, modify, or gain a closer look over your stats, the database file containing all of your stats should be in the config directory for your respective OS.

| Platform | Value                                              | Example                                            |
| -------- | -------------------------------------------------- | -------------------------------------------------- |
| Linux    | `$XDG_DATA_HOME` or `$HOME`/.local/share/mpdtrackr | /home/alice/.local/share/mpdtrackr                 |
| macOS    | `$HOME`/Library/Application Support/mpdtrackr      | /Users/Alice/Library/Application Support/mpdtrackr |
| Windows  | `{FOLDERID_RoamingAppData}`\mpdtrackr              | C:\Users\Alice\AppData\Roaming\mpdtrackr           |

## Usage

```
MPD listening time tracker with versatile stats and sync

Usage: mpdtrackr <COMMAND>

Commands:
  run     Run the daemon
  print   Print listening stats to stdout with formatting options
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
