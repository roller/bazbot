
# Bazbot

An irc bot that generates markov chain phrases.

# Purpose

This is a rust programming language[1] learning project.  It is not intended a
demonstration of good code quality or best practices.  Feedback is welcome.

# Usage

```
USAGE:
    bazbot [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE.json>    Read config from json file (defaults to env var
                                BAZBOT_CONFIG or bazbot.config).

SUBCOMMANDS:
    complete    Run a markov chain starting with args
    help        Prints this message or the help message of the given
                subcommand(s)
    irc         Interact on irc channels
    migrate     Create or sync database against current code
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
    RUST_LOG        - debug, info, warn, error
    BAZBOT_CONFIG   - default json config file location
    BAZBOT_WORDS    - default sqlite database location
```

# History

Bazbot was inspired by the benzo irc bot[2] named baz.

# Contributing

Contributions to this library would be immensely appreciated. It should be
noted that as this is a public domain project, any contributions will thus be
released into the public domain as well.

[1]: https://www.rust-lang.org/
[2]: http://benzo.sourceforge.net/
