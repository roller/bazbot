
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

One dependency not handled by cargo is the openssl library.  See the
[rust-openssl][3] project for information on compiling for your platform.


# Usage

```
USAGE:
    bazbot [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE.json>    Read config from json file (defaults to env var
                                BAZBOT_CONFIG or bazbot.json).

SUBCOMMANDS:
    add         Add a phrase to the markov words database
    complete    Run a markov chain starting with args
    help        Prints this message or the help message of the given
                subcommand(s)
    irc         Interact on irc channels
    read        Read text file with one phrase per line into markov
                database
    summary     Summarize database

Files:
    bazbot.json
        A valid config file is required to connect to irc.
        Words db location may be configured as options.words, eg:
            { "options": { "words": "bazbot.db" } }
        See irc library documentation for more information:
           https://github.com/aatxe/irc
    bazbot.db
        sqlite file with bazbot's brain

Environment
    Additional environment will be read from .env
    RUST_LOG        - debug, info, warn, error (default info)
    BAZBOT_CONFIG   - default json config file location
    BAZBOT_WORDS    - default sqlite database location
```

Example minimum `bazbot.json` config file:

``` json
{
    "nickname": "bazbot",
    "server": "localhost",
    "port": 6667,
    "channels": ["#testbazbot"],
    "options": {
        "words": "bazbot.db"
    }
}
```

# History

Bazbot was inspired by the [benzo irc bot][4] named baz.

[1]: https://www.rust-lang.org/
[2]: https://www.rustup.rs/
[3]: https://github.com/sfackler/rust-openssl
[4]: http://benzo.sourceforge.net/
