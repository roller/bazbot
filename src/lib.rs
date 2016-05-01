extern crate rusqlite;

use std::env;
use rusqlite::{Result, Connection};

/*
#[derive(Debug)]
enum BazError {
    Rusqlite(rusqlite::Error)
}
type Result<T> = Result<T, Error>;
*/

#[derive(Debug)]
pub struct Baz {
    db: Connection
}


impl Baz {
    pub fn new() -> Baz {
        let db_url = env::var("WORDS_DB")
            .expect("WORDS_DB must be set");
        Baz {
            db: Connection::open(db_url)
                .expect("Could not open database")
        }
    }
    pub fn summary(&self) {
        println!("This is the summary of {:?}", self);
        if let Ok(words) = self.db.query_row(
            "select count(*) from words", &[], |row| {
                row.get::<i64>(0)
            }) {
            println!("Words: {}", words);
        }
        if let Ok(phrases) = self.db.query_row(
            "select count(*) from phrases", &[], |row| {
                row.get::<i64>(0)
            }) {
            println!("Phrases: {}", phrases);
        }
        self.query_next_word("This","is");
    }

    pub fn query_next_word(&self, word1: &str, word2: &str) {
        let res1 = self.get_word_id(word1);
        let res2 = self.get_word_id(word2);
        if let (Ok(Some(id1)), Ok(Some(id2))) = (res1, res2) {
            println!("Found words {}, {}", id1, id2);
            match self.get_pair_freq(id1,id2) {
                Ok(Some(freq)) => {
                    println!("Got frequency {}", freq);
                    let next = self.get_next_word(id1,id2,2)
                        .expect("Couldn't find a word")
                        .expect("There was no word");
                    let spelling = self.get_spelling(next);
                    println!("Found a word {:?}", spelling);
                }
                Ok(None) => {
                    println!("Words not found");
                }
                Err(e) => {
                    println!("Couldn't query: {:?}", e);
                }
            }
        }
    }

    fn get_next_word(&self, id1: i64, id2: i64, count: i64) 
        -> Result<Option<i64>> {
        let mut freq_count: i64 = count;
        let mut stmt = try!(self.db.prepare(
            "select freq, word3 from phrases where word1=? and word2=?"));
        let rows = try!(stmt.query(&[&id1, &id2]));
        for result_row in rows {
            let row = try!(result_row);
            let freq: i64 = row.get(0);
            if freq_count <= freq {
                return Ok(row.get::<Option<i64>>(1));
            }
            freq_count -= freq;
        }
        Ok(None)
    }

    fn get_pair_freq(&self, id1: i64, id2: i64) -> Result<Option<i64>> {
        self.db.query_row(
            "select sum(freq) from phrases where word1=? and word2=?",
            &[&id1, &id2],
            |row| { row.get::<Option<i64>>(0) } )
    }

    fn get_word_id(&self, spelling: &str) -> Result<Option<i64>> {
        self.db.query_row(
            "select word_id from words where spelling=?",
            &[&spelling],
            |row| { row.get::<Option<i64>>(0) } )
    }

    fn get_spelling(&self, word_id: i64) -> Result<Option<String>> {
        self.db.query_row(
            "select spelling from words where word_id=?",
            &[&word_id],
            |row| { row.get::<Option<String>>(0) } )
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
