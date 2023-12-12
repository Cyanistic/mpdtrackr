<div align="center">

**A CLI [MPD](https://www.musicpd.org/) listening time tracker with versatile stats.**

[Features](#features) •
[Installation](#installation) •
[Getting Started](#getting-started) •
[Usage](#usage)

</div>

## Features

- Keeps track of which songs are playing, how long they have been playing each day, and maintains historical playtime data to view trends over time
- Displays listening statistics directly in the terminal for easy access and analysis
  - Allows users to group stats by time periods (day, week, month, year, all-time) and different fields (artist, album, title, genre)
  - Permits sorting of statistics based on multiple criteria for in-depth analysis
- Facilitates hassle-free transfer of stats across devices using an SQLite database, compatible with tools like [syncthing](https://syncthing.net/)

## installation

Note: I have only tested mpdtrackr on Linux.

<details>
   <summary>Windows</summary>

> Download the latest release of mpdtrackr from the releases page.

</details>

<details>
   <summary>macOS/Linux</summary>

> You can use the [install.sh](https://github.com/Cyanistic/mpdtrackr/blob/master/install.sh) script to install and download the [latest release](https://github.com/Cyanistic/mpdtrackr/releases/latest) of mpdtrackr and enable the systemd service.
>
> The following command downloads and executes the script:
>
> ```
> curl --proto '=https' -sSf 'https://raw.githubusercontent.com/Cyanistic/mpdtrackr/master/install.sh' | sh
> ```
>
> If you are unhappy with mpdtrackr you can also uninstall it using a similar command
>
> ```
> curl --proto '=https' -sSf 'https://raw.githubusercontent.com/Cyanistic/mpdtrackr/master/install.sh' | sh -s uninstall
> ```
>
> If you prefer to avoid using scripts you can do the following
>
> 1. Download the latest release for your specific OS from the releases page
> 2. Make the file executable using `chmod +x`
> 3. Move the file into `/usr/bin`

</details>

## Getting Started

Ensure that you've installed the latest release from the [releases](https://github.com/Cyanistic/mpdtrackr/releases/latest) page and followed the [installation guide](#installation).

Using mpdtrackr is as simple as running `mpdtrackr run` after starting up your MPD server.

In order for mpdtrackr to properly keep track of songs, your music files should have the proper title, artist, genre, etc. tags. Otherwise the application has to attempt to parse the title and artist from the file name, which can be inaccurate, and will not have any data for the album or genre.

To make sure that you never forget to start up the mpdtrackr daemon before listening to music, you probably want to make sure it runs on startup. There is a minimal `mpdtrackr.service` file provided to let you do that, provided that you use systemd. Otherwise you probably already know how to make a command run on startup using your window-manager configuration or something of the sort.

If you want to transfer, modify, or gain a closer look over your stats, the database file containing all of your stats should be in the data directory for your respective OS.

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
