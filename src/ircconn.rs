
extern crate irc;
use self::irc::client::prelude::{
    Config,
    Client,
    Command,
    Prefix,
    Message
};
use irc::error::Result;
use futures::*;

use std::cell::RefCell;
use crate::markov_words;
use crate::markov_words::WordsDb;
use futures::executor::block_on;
// use self::irc::proto::prefix::Prefix::ServerName;

pub struct IrcConn {
    words: RefCell<Box<WordsDb>>,
    client: Client
}

impl IrcConn {
    pub fn new(words: WordsDb, client: Client) -> IrcConn {
        IrcConn {
            words: RefCell::new(Box::new(words)),
            client
        }
    }

    pub fn new_from_config(words: WordsDb, config: Config) -> IrcConn {
        let client = block_on(async {
            Client::from_config(config).await.expect("Client from config")
        });

        IrcConn {
            words: RefCell::new(Box::new(words)),
            client
        }
    }

    pub async fn run(&mut self) {
        self.run_irc()
            .await
            .unwrap_or_else(|e| error!("Error running irc: {:?}", e));
    }

    async fn run_irc(&mut self) -> Result<()>
    {
        self.client.identify()?;
        debug!("Identified.");
        let mut stream = self.client.stream()?;
        while let Some(message) = stream.next().await.transpose()? {
            self.handle_message(&message);
        }
        Ok(())
    }

    fn handle_message(&self, msg: &Message) {
        debug!("Handle message: {:?}", msg);
        if let Some(ref prefix) = msg.prefix {
            match msg.command {
                Command::JOIN(_, _, _) => info!("join channel {:?}", msg),
                Command::PRIVMSG(ref target,ref text) =>
                    self.privmsg(prefix, target, text),
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
                let res = self.client.send_privmsg(target, &response);
                if let Err(x) = res {
                    error!("Uhoh sending msg: {:?}",x);
                }
            }
            Err(e) => error!("Uhoh: {:?}", e)
        }
    }

    fn privmsg(&self, prefix: &Prefix, target: &str, text: &str) {
        info!("msg {:?} {} {}", prefix, target, text);
        let phrase = markov_words::tokenize_phrase(text);
        let nearby = markov_words::find_nearby(self.client.current_nickname(), &phrase);
        if nearby.is_empty() {
            let owned_phrase: Vec<String> = phrase.iter().map(ToString::to_string).collect();
            let words = self.words.borrow_mut();
            if let Err(e) = words.add_phrase(&owned_phrase) {
                error!("Error adding line: {}", e);
            }
        } else {
            self.respond_to_name(target, nearby);
        }
    }

}
