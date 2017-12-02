extern crate irc;
use self::irc::client::prelude::*;

use std::cell::RefCell;
use markov_words;
use markov_words::WordsDb;

pub struct IrcConn {
    words: RefCell<Box<WordsDb>>,
    server: IrcServer
}

impl IrcConn {
    pub fn new(words: WordsDb, server: IrcServer) -> IrcConn {
        IrcConn {
            words: RefCell::new(Box::new(words)),
            server: server
        }
    }

    pub fn new_from_config(words: WordsDb, config: Config) -> IrcConn {
        let server = IrcServer::from_config(config).unwrap();
        IrcConn {
            words: RefCell::new(Box::new(words)),
            server: server
        }
    }

    pub fn run(&self) {
        self.server.identify().unwrap();
        debug!("identified.");
        self.server.for_each_incoming(|ok_msg| self.handle_message(&ok_msg)).unwrap()
    }

    fn handle_message(&self, msg: &Message){
        if let Some(ref prefix) = msg.prefix {
            let prefix_info = PrefixInfo::new(prefix);
            match msg.command {
                Command::JOIN(_, _, _) => info!("join channel {:?}", msg),
                Command::PRIVMSG(ref target,ref text) => self.privmsg(&prefix_info, target, text),
                _ => debug!("ignore: {:?}", msg)
            }
        } else {
            // PING
            debug!("ignore no prefix: {:?}", msg);
        }
    }

    fn respond_to_name(&self, target: &str, nearby: Vec<Vec<&str>>) {
        debug!("nearby words: {:?}", nearby);
        let words = self.words.borrow_mut();
        let result_words = words.new_complete_middle_out(nearby);
        match result_words {
            Ok(words) => {
                let response = markov_words::join_phrase(vec![], words);
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
        let phrase = markov_words::tokenize_phrase(text);
        let nearby = markov_words::find_nearby(self.server.current_nickname(), &phrase);
        if nearby.is_empty() {
            let owned_phrase: Vec<String> = phrase.iter().map(|s| s.to_string()).collect();
            let words = self.words.borrow_mut();
            if let Err(e) = words.add_phrase(&owned_phrase) {
                error!("Error adding line: {}", e);
            }
        } else {
            self.respond_to_name(target, nearby);
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
        let mut name_host = s.splitn(2,'@');
        let name = name_host.next().unwrap_or("");
        let host = name_host.next();
        let mut name_user = name.splitn(2,'!');
        PrefixInfo {
            name: name_user.next().unwrap_or(""),
            user: name_user.next(),
            host: host
        }
    }
}
