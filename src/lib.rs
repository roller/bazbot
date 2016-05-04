extern crate rusqlite;
extern crate rand;

use std::env;
use rusqlite::{Result, Connection};
use rusqlite::types::ToSql;
use rand::random;

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

struct NamedParam<'a> {
    field: String,
    value: &'a ToSql
}
impl<'a> NamedParam<'a> {
    fn new(field: &str, value: &'a ToSql) -> NamedParam<'a>{
        NamedParam {
            field: field.to_string(),
            value: value
        }
    }
    fn assigns(params: &Vec<NamedParam>) -> Vec<String> {
        params.iter().map(|w| { format!("{}=?", w.field) }).collect()
    }
    fn values(params: &'a Vec<NamedParam>) -> Vec<&'a ToSql> {
        params.iter().map(|w| w.value).collect()
    }
}



impl Baz {
    pub fn new(db_url: String) -> Baz {
        Baz {
            db: Connection::open(db_url)
                .expect("Could not open database")
        }
    }
    pub fn new_from_env() -> Baz {
        let db_url = env::var("WORDS_DB")
            .expect("WORDS_DB must be set");
        Baz::new(db_url)
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
        self.query_next_word("That","was");
    }

    pub fn query_next_word(&self, word1: &str, word2: &str) {
        let res1 = self.get_word_id(word1);
        let res2 = self.get_word_id(word2);
        if let (Ok(Some(id1)), Ok(Some(id2))) = (res1, res2) {
            println!("Found words {}, {}", id1, id2);
            let filter_word_id = vec![
                NamedParam::new("word1", &id1),
                NamedParam::new("word2", &id2)
            ];
            match self.get_pair_freq_where_filter(&filter_word_id) {
                Ok(Some(freq)) => {
                    println!("Got frequency {}", freq);
                    let pick = random::<i64>() % freq + 1;
                    let next = self.get_next_word(id1,id2,pick)
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


    fn get_next_word(&self, id1: i64, id2: i64, pick: i64)
        -> Result<Option<i64>> {
        let mut pick_count: i64 = pick;
        let mut stmt = try!(self.db.prepare(
            "select freq, word3 from phrases where word1=? and word2=?"));
        let rows = try!(stmt.query(&[&id1, &id2]));
        for result_row in rows {
            let row = try!(result_row);
            let freq: i64 = row.get(0);
            if pick_count <= freq {
                return Ok(row.get::<Option<i64>>(1));
            }
            pick_count -= freq;
        }
        Ok(None)
    }

    fn get_pair_freq_where_filter(&self, filter: &Vec<NamedParam>) -> Result<Option<i64>> {
        let wheres = NamedParam::assigns(filter);
        let values = NamedParam::values(filter);
        let sql_where = if wheres.as_slice().is_empty() {
            String::from("")
        } else {
            format!("where {}", wheres.join(&String::from(" and ")))
        };
        let sql = format!("select sum(freq) from phrases {}", sql_where);
        println!("Running sql: {}", sql);

        self.db.query_row(&sql, values.as_slice(),
            |row| row.get::<Option<i64>>(0))
    }


    fn get_word_id(&self, spelling: &str) -> Result<Option<i64>> {
        self.db.query_row(
            "select word_id from words where spelling=?",
            &[&spelling], |row| row.get::<Option<i64>>(0))
    }

    fn get_spelling(&self, word_id: i64) -> Result<Option<String>> {
        self.db.query_row(
            "select spelling from words where word_id=?",
            &[&word_id], |row| row.get::<Option<String>>(0))
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
