extern crate irc;
use self::irc::client::prelude::*;
use std::io::Result;

use markov_words;
use markov_words::WordsDb;

pub struct IrcConn {
    words: WordsDb,
    server: IrcServer
}

impl IrcConn {
    pub fn new(words: WordsDb, server: IrcServer) -> IrcConn {
        IrcConn {
            words: words,
            server: server
        }
    }

    pub fn new_from_config(words: WordsDb, config: &str) -> IrcConn {
        let server = IrcServer::new(config).unwrap();
        IrcConn {
            words: words,
            server: server
        }
    }
    pub fn new_from_env(words: WordsDb) -> IrcConn {
        unimplemented!();
    }

    pub fn run(&self) {
        let id = self.server.identify().unwrap();
        debug!("identified: {:?}", id);
        for message in self.server.iter() {
            match message {
                Ok(ok_msg) => self.handle_message(&ok_msg),
                Err(e) => error!("error: {:?}", e)
            }
        }
    }

    fn handle_message(&self, msg: &Message){
        if let Some(ref prefix) = msg.prefix {
            let prefix_info = PrefixInfo::new(prefix);
            match msg.command {
                Command::JOIN(ref channel, _, _) => info!("join channel {:?}", msg),
                Command::PRIVMSG(ref target,ref text) => self.privmsg(&prefix_info, &target, &text),
                _ => debug!("ignore: {:?}", msg)
            }
        } else {
            // PING
            debug!("ignore no prefix: {:?}", msg);
        }
    }

    fn respond_to_name(&self, prefix: &PrefixInfo, target: &str, text: &str) {
        let prefix_words: Vec<String> = text.split_whitespace()
            .take_while(|x| !x.starts_with("baz"))
            .take(2)
            .map(|x| x.to_owned())
            .collect();
        let prefix_words = if prefix_words.len() == 0 {
            vec!["".to_owned()] // begin phrase sentinel
        } else {
            prefix_words
        };

        debug!("prefix_words: {:?}", prefix_words);
        let result_words = self.words.complete(&prefix_words);
        match result_words {
            Ok(words) => {
                let response = markov_words::join_phrase(prefix_words, words);
                let res = self.server.send_privmsg(target, &response);
                if let Err(x) = res {
                    error!("Uhoh sending msg: {:?}",x);
                }
            }
            Err(e) => error!("Uhoh: {:?}", e)
        }
    }

    fn privmsg(&self, prefix: &PrefixInfo, target: &str, text: &str) {
        info!("msg {:?} {} {}", prefix, target, text);
        if text.split_whitespace().any(|x| x.starts_with("baz"))  {
            self.respond_to_name(prefix, target, text);
        }
    }

}

#[derive(Debug)]
struct PrefixInfo<'a> {
    // RFC2812 defines this as
    // prefix     =  servername / ( nickname [ [ "!" user ] "@" host ] )
    // name can be servername or nickname
    name: &'a str,
    user: Option<&'a str>,
    host: Option<&'a str>
}

impl<'a> PrefixInfo<'a> {
    // yeah, no idea how to parse things
    fn new(s: &'a str) -> PrefixInfo<'a> {
        let mut name_host = s.splitn(2,"@");
        let name = name_host.next().unwrap_or("");
        let host = name_host.next();
        let mut name_user = name.splitn(2,"!");
        PrefixInfo {
            name: name_user.next().unwrap_or(""),
            user: name_user.next(),
            host: host
        }
    }
}
