extern crate dotenv;
extern crate rusqlite;


use dotenv::dotenv;
use std::env;
use rusqlite::Connection;
use rusqlite::Result;

#[derive(Debug)]
pub struct Baz {
    db: Connection
}

impl Baz {
    pub fn new() -> Baz {
        let db_url = env::var("WORDS_DB")
            .expect("WORDS_DB must be set");
        Baz {
            db: Connection::open(db_url).expect("Could not open database")
        }
    }
    pub fn summary(&self) {
        println!("This is the summary {:?}", self);
    }
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
    #[test]
    fn whatever() {
        let c = establish_connection();
    }
}
