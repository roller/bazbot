
extern crate irc;
use self::irc::client::prelude::*;
use markov_words::WordsDb;

pub struct BazIrc {
    words: WordsDb,
    server: IrcServer
}

impl BazIrc {
    pub fn new(words: WordsDb, server: IrcServer) -> BazIrc {
        BazIrc {
            words: words,
            server: server
        }
    }

    pub fn new_from_config(words: WordsDb, config: &str) -> BazIrc {
        let server = IrcServer::new(config).unwrap();
        BazIrc {
            words: words,
            server: server
        }
    }
    pub fn new_from_env(words: WordsDb) -> BazIrc {
        unimplemented!();
    }
    pub fn run(&self) {
        unimplemented!();
    }
}
