extern crate irc;
use self::irc::client::prelude::*;

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

    pub fn new_from_config(words: WordsDb, config: Config) -> IrcConn {
        let server = IrcServer::from_config(config).unwrap();
        IrcConn {
            words: words,
            server: server
        }
    }

    pub fn run(&self) {
        self.server.identify().unwrap();
        debug!("identified.");
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
                Command::JOIN(_, _, _) => info!("join channel {:?}", msg),
                Command::PRIVMSG(ref target,ref text) => self.privmsg(&prefix_info, target, text),
                _ => debug!("ignore: {:?}", msg)
            }
        } else {
            // PING
            debug!("ignore no prefix: {:?}", msg);
        }
    }

    fn respond_to_name(&self, target: &str, surround: &[&str]) {
        debug!("surround words: {:?}", surround);
        let result_words = self.words.complete_middle_out(surround);
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
        if let Some(surround) = markov_words::find_match_surround(self.server.current_nickname(), &phrase) {
            self.respond_to_name(target, &surround);
        } else {
            let owned_phrase = phrase.iter().map(|s| s.to_string()).collect();
            if let Err(e) = self.words.add_phrase(owned_phrase) {
                error!("Error adding line: {}", e);
            }
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
