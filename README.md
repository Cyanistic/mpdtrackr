# mpdtrackr
## Track your [mpd](https://www.musicpd.org/) listening time locally on a per artist and per song basis.
### Command line application I made because I prefer using mpd for music but got jealous of all the spotify people who got to see what they listened to the most.

#### Current Features
* Prints database of songs to stdout
* Keeps track of which songs are playing and how long they have been playing
* Allows for easy configuration of mpd and mongodb ports in the config file (~/.config/mpdtrackr/config.json on Linux or %appdata%\mpdtrackr\config.json on Windows (probably won't work on windows tho))
* Import/Export databases to help keep track across devices

#### Planned Features
* Still thinking

#### Usage
```
A simple MPD tracker

Usage: mpdtrackr [OPTIONS]

Options:
  -i, --import [<IMPORT>...]  Import collections from files (ex: artists.json will be imported into into the "artists" collection) Files must be in .json format and have the .json extension to be properly imported
  -l, --logging               Run the tracker while printing logs to stdout
  -p, --print                 Print the database to stdout
  -o, --output [<OUTPUT>...]  Directories to output the database to. Output files will be in .json format
  -h, --help                  Print help (see more with '--help')
  -V, --version               Print version
```

Still fairly scuffed since I haven't learned how to work with threads and this is my first rust project that isn't just copying a coreutil.
