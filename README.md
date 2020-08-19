
# Bazbot

An irc bot that generates markov chain phrases.

# Purpose

This is a [rust programming language][1] learning project.  It is not intended a
demonstration of good code quality or best practices.  Feedback is welcome.

# Installation

Sorry, there's no installer.  [Rust up][2] and build with cargo.

```
cargo build --release
```

Dependencies on Debian and Ubuntu:

```
sudo apt install libsqlite3-dev pkg-config libssl-dev
```

For other platforms, see the crate documentation:

 - [rusqlite][3]
 - [rust-openssl][4]

# Usage

```
Bazbot Blabberbot 0.2.6

USAGE:
    bazbot [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE.toml>    Read config from json file (defaults to env var BAZBOT_CONFIG or bazbot.json).

SUBCOMMANDS:
    add         Add a phrase to the markov words database
    complete    Run a markov chain matching args around _
    help        Prints this message or the help of the given subcommand(s)
    irc         Interact on irc channels
    read        Read text file with one phrase per line into markov database
    summary     Summarize database


Files:
    bazbot.toml
    or
    bazbot.json
        A valid config file is required to connect to irc.
        See irc library documentation for more information:
           https://github.com/aatxe/irc
        The following options are supported:
         - words - sqlite database to store phrases
         - learn - learn new phrases from irc
        e.g. toml:
        options: { words="bazbot.db" }
        e.g. json:
        { "options": { "words": "bazbot.db" } }
    bazbot.db
        default sqlite file storing phrases

Environment
    Additional environment will be read from .env
    RUST_LOG        - debug, info, warn, error (default info)
    BAZBOT_CONFIG   - default json config file location
    BAZBOT_WORDS    - default sqlite database location
```

Example `bazbot.toml` config file:

``` toml
nickname: "bazbot",
server: "localhost",
port: 6667,
channels: ["#testbazbot"],

[options]
## Store phrases in sqlite database
words = "bazbot.db"

## Learn new phrases, set "false" when using static phrase
## database that shouldn't be polluted with irc conversation
learn = "true"
```


The following options in the irc are supported (with defaults shown):

``` toml
```

# History

Bazbot was inspired by the [benzo irc bot][5] named baz.

[1]: https://www.rust-lang.org/
[2]: https://www.rustup.rs/
[3]: https://github.com/rusqlite/rusqlite
[4]: https://docs.rs/openssl/
[5]: http://benzo.sourceforge.net/
