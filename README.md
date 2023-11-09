<div align="center">

**A CLI [MPD](https://www.musicpd.org/) listening time tracker with versatile stats and sync.**

[Features](#features) •
[Getting Started](#getting-started) •
[Usage](#usage)

</div>

## Features

- Keeps track of which songs are playing, how long they have been playing each day, and maintains historical playtime data to view trends over time.
- Prints database of stats to stdout
  - Stats can be grouped by time periods such as day, week, month, year, and all-time
  - Stats can also be sorted in by multiple parameters
- Easily transfer stats across devices using software like [syncthing](https://syncthing.net/) due to the fact that stats are saved in a SQLite database

## Getting Started

Ensure that you've installed the latest release from the [releases](https://github.com/Cyanistic/mpdtrackr/releases/latest) page.

Using mpdtrackr is as simple as running `mpdtrackr run` after starting up your MPD server.

To make sure that you never forget to start up the mpdtrackr daemon before listening to music, you probably want to make sure it runs on startup. There is a minimal `mpdtrackr.service` file provided to let you do that provided, that you use systemd. Otherwise you probably already know how to make a command run on startup using your window-manager configuration or something of the sort.

If you want to transfer, modify, or gain a closer look over your stats, the database file containing all of your stats should be in the config directory for your respective OS.

| Operating System | Path                                                  | Example                                            |
| ---------------- | ----------------------------------------------------- | -------------------------------------------------- |
| Windows          | {FOLDERID_RoamingAppData}\mpdtrackr                   | C:\Users\Alice\AppData\Roaming\mpdtrackr           |
| macOS            | $HOME/Library/Application Support/mpdtrackr           | /Users/Alice/Library/Application Support/mpdtrackr |
| Linux            | $XDG_CONFIG_HOME/mpdtrackr or $HOME/.config/mpdtrackr | /home/alice/.config/mpdtrackr                      |

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
