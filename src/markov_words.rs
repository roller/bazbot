extern crate rusqlite;
extern crate rand;

use migration;
use std::env;
use rusqlite::{Result, Connection};
use rusqlite::types::ToSql;
use rand::random;

// utility construct to pass names names with values
struct NamedParam<'a> {
    field: String,
    value: Box<ToSql + 'a>
}

impl<'a> NamedParam<'a> {
    fn new(field: &str, value: Box<ToSql + 'a>) -> NamedParam<'a>{
        NamedParam {
            field: field.to_string(),
            value: value
        }
    }
    fn assigns(params: &Vec<NamedParam>) -> Vec<String> {
        params.iter().map(|w| { format!("{}=?", w.field) }).collect()
    }
    fn values(params: &'a Vec<NamedParam>) -> Vec<&'a ToSql> {
        params.iter().map(|w| &*w.value).collect()
    }
}

pub struct ChainIter<'a> {
    baz: &'a WordsDb,
    prefix: Vec<i64>
}

impl<'a> ChainIter<'a> {
    fn push(&mut self, n: i64){
        // keep at most 2 elements on prefix
        while self.prefix.len() > 1 {
            self.prefix.remove(0);
        }
        self.prefix.push(n);
    }
}

impl<'a> Iterator for ChainIter<'a> {
    type Item = i64;
    // TODO: Should this be a Option<Result<i64>> ?
    fn next(&mut self) -> Option<i64> {
        let res = self.baz.complete_int(self.prefix.as_slice());
        match res {
            Ok(Some(n)) => {
                self.push(n);
                Some(n)
            },
            Ok(None) => None,
            Err(e) => {
                println!("Ending early due to {:?}", e);
                None
            }
        }
    }
}

// join two vector phrases with spaces
fn join_phrase(phrase1: Vec<String>, phrase2: Vec<String>) -> String {
    phrase1.into_iter()
        .chain(phrase2)
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .as_slice()
        .join(" ")
}

#[derive(Debug)]
pub struct WordsDb {
    db: Connection
}


impl WordsDb {
    pub fn new(db_url: String) -> WordsDb {
        println!("Open db {}", db_url);
        WordsDb {
            db: Connection::open(db_url)
                .expect("Could not open database")
        }
    }
    pub fn new_from_env() -> WordsDb {
        let db_url = env::var("WORDS_DB")
            .expect("WORDS_DB must be set");
        WordsDb::new(db_url)
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
    }

    pub fn migrate(&self) -> Result<()> {
        migration::migrate(&self.db)
    }

    pub fn complete_int(&self, prefix: &[i64]) -> Result<Option<i64>> {
        let fields: Vec<&str> = vec!["word1","word2"];
        let filter: Vec<NamedParam> = fields.iter().zip(prefix).map(|(fname, pid)| {
            NamedParam::new(&*fname, Box::new(*pid))
        }).collect();
        match self.get_pair_freq_where_filter(&filter) {
            Ok(Some(freq)) => {
                let pick = random::<i64>().abs() % freq + 1;
                self.get_next_word_filter(&filter, pick)
            }
            result => result
        }
    }

    pub fn complete_iter(&self, prefix_words: &Vec<String>) -> ChainIter {
        let prefix_ints: Vec<i64> = prefix_words.iter().flat_map(|pword| {
            let res = self.get_word_id(&pword);
            match res {
                Ok(Some(word_id)) => vec![ word_id ],
                Ok(None) => {
                    println!("ignoring null value for {:?}", pword);
                    vec![]
                }
                Err(err) => {
                    println!("ignoring an error {:?}", err);
                    vec![]
                }
            }
        }).collect();
        ChainIter {
            baz: self,
            prefix: prefix_ints
        }
    }

    pub fn complete(&self, prefix: &Vec<String> ) -> Result<Vec<String>> {
        let words: Vec<Option<String>> =
            try!(self.complete_iter(prefix)
                     .map(|id| self.get_spelling(id))
                     .collect());
        return Ok(words.into_iter().flat_map(|x| x).collect())
    }

    pub fn print_complete(&self, prefix: Vec<String> ) {
        let result_words = self.complete(&prefix);
        match result_words {
            Ok(words) => println!("baz: {}", join_phrase(prefix, words)),
            Err(e) => println!("Uhoh: {:?}", e)
        }
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

        self.db.query_row(&sql, values.as_slice(),
            |row| row.get::<Option<i64>>(0))
    }

    fn get_next_word_filter(&self, prefix_filter: &Vec<NamedParam>, pick: i64)
        -> Result<Option<i64>> {
        let wheres = NamedParam::assigns(prefix_filter);
        let values = NamedParam::values(prefix_filter);
        // retrieve column based on how many words in prefix
        let select_field_names = vec![ "word1", "word2", "word3" ];
        let select_field = select_field_names[ prefix_filter.len() ];
        let sql_where = if prefix_filter.is_empty() {
            "".to_string()
        } else {
            format!("where {}", wheres.join(&String::from(" and ")))
        };
        let sql = format!(
            // note: this code was lightly tested, but it seems
            //       that summing in sqlite engine is actually slower
            //       than in rust
            // "select sum(freq), {} from phrases {} group by {}",
            // select_field, sql_where, select_field);
            "select freq, {} from phrases {}",
            select_field, sql_where);

        let mut pick_count: i64 = pick;
        let mut stmt = try!(self.db.prepare(&sql));

        let rows = try!(stmt.query(&values));
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
        let c = super::WordsDb::new(":memory:".to_string());
        c.summary()
    }
}