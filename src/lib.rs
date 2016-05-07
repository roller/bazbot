extern crate rusqlite;
extern crate rand;

use std::env;
use rusqlite::{Result, Connection,Error};
use rusqlite::types::ToSql;
use rand::random;

/*
#[derive(Debug)]
enum BazError {
    Rusqlite(rusqlite::Error)
}
type Result<T> = Result<T, Error>;
*/

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

#[derive(Debug)]
struct Migration<'a> {
    m_id: &'a str,
    m_sql: &'a str,
}

#[derive(Debug)]
pub struct Baz {
    db: Connection
}
impl Baz {
    pub fn new(db_url: String) -> Baz {
        println!("Open db {}", db_url);
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
    }

    // Use a different technique for the very first migration:
    // check sqlite_master table for the existance of the table
    fn base_migration(&self) -> Result<()> {
        let base = Migration {
            m_id: "init",
            m_sql: "create table migrations ( m_id primary key );"
        };
        let sql = "select name from sqlite_master where type='table' and name='migrations';";
        let res = self.db.query_row(sql, &[], |row| ());
        match res {
            Ok(()) => Ok(()),
            Err(Error::QueryReturnedNoRows) => self.run_migration(&base),
            e @ Err(_) => e
        }
    }

    pub fn migrate(&self) -> Result<()> {
        try!(self.base_migration());
        let migrations = vec![
            Migration {
                m_id: "words_and_phrases_init",
                m_sql: "
                CREATE TABLE words (word_id integer primary key autoincrement, spelling text not null);
                CREATE TABLE phrases (
                    word1 integer not null, word2 integer not null, word3 integer not null, freq integer
                );
                insert into words (word_id, spelling) values (0,'');
                CREATE UNIQUE INDEX idx_words on words (word_id);
                CREATE UNIQUE INDEX idx_spelling on words (spelling);
                CREATE UNIQUE INDEX idx_phrases_u on phrases (word1,word2,word3);"
            }
        ];
        for migration in migrations {
            if !try!(self.check_migration(&migration)) {
                try!(self.run_migration(&migration))
            }
        }
        Ok(())
    }

    fn run_migration(&self, migration: &Migration) -> Result<()> {
        println!("run migration: {:?}", migration.m_id);
        try!(self.db.execute_batch(migration.m_sql));
        try!(self.db.execute(
            "insert into migrations (m_id) values (?)",
            &[ &migration.m_id ]
        ));
        Ok(())
    }

    // returt true if migration is already logged in db
    fn check_migration(&self, migration: &Migration) -> Result<bool> {
        let check_sql = "select 1 from migrations where m_id = ?";
        let params : Vec<&ToSql> = vec![&migration.m_id];
        let res = self.db.query_row(&check_sql, &params,
            |row| row.get::<Option<i64>>(0));
        println!("Migration check {:?}: {:?}",migration.m_id ,res);
        match res {
            Err(Error::SqliteFailure(_,_)) => Ok(false), // returned when no migration table
            Err(Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(e),
            Ok(_) => Ok(true)
        }
    }

    pub fn complete(&self, prefix: Vec<String> ) {
        let fields: Vec<&str> = vec!["word1","word2"];

        let filter: Vec<NamedParam> = fields.iter().zip(prefix).flat_map(|(field_name, prefixword)| {
            let res = self.get_word_id(&prefixword);
            match res {
                Ok(Some(word_id)) => vec![ NamedParam::new(&*field_name, Box::new(word_id)) ],
                Ok(None) => {
                    println!("ignoring null value for {:?}", prefixword);
                    vec![]
                }
                Err(err) => {
                    println!("ignoring an error {:?}", err);
                    vec![]
                }
            }
        }).collect();

        match self.get_pair_freq_where_filter(&filter) {
            Ok(Some(freq)) => {
                println!("Got frequency {}", freq);
                let pick = random::<i64>().abs() % freq + 1;
                let next = self.get_next_word_filter(&filter, pick)
                    .expect("Error during query");
                if let Some(word) = next {
                    let spelling = self.get_spelling(word);
                    println!("Found a word {:?}", spelling);
                } else {
                    println!("Found Null")
                }
            }
            Ok(None) => {
                println!("Words not found");
            }
            Err(e) => {
                println!("Couldn't query: {:?}", e);
            }
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
        println!("Running sql: {}", sql);

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
            String::from("")
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
        println!("Find pick {} Running sql: {}", pick, sql);

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
        let c = super::Baz::new(":memory:".to_string());
        c.summary()
    }
}
