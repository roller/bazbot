extern crate rusqlite;
extern crate rand;
extern crate irc;

use migration;
use std::{env,fs};
use std::io::{BufRead,BufReader};
use rusqlite::{Result, Connection,Error};
use rusqlite::types::ToSql;
use rand::random;
use self::irc::client::data::config::Config;

enum WordField {
    Word1,
    Word2,
    Word3,
}

impl WordField {
    fn to_str(self) -> &'static str {
        match self {
            WordField::Word1 => "word1",
            WordField::Word2 => "word2",
            WordField::Word3 => "word3"
        }
    }
}

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
    words: &'a WordsDb,
    // database fields to filter or select, corresponding to how many filter
    filter_fields: Vec<&'a str>,
    filter_values: Vec<i64>,
    count: i64
}

impl<'a> ChainIter<'a> {
    fn push(&mut self, n: i64){
        // keep at most 2 elements to filter
        while self.filter_values.len() > 1 {
            self.filter_values.remove(0);
        }
        self.filter_values.push(n);
    }
}

impl<'a> Iterator for ChainIter<'a> {
    type Item = i64;
    // TODO: Should this be a Option<Result<i64>> ?
    fn next(&mut self) -> Option<i64> {
        let filter: Vec<NamedParam> = self.filter_fields.iter()
                                       .zip(self.filter_values.iter())
                                       .map(|(f,v)| NamedParam::new(*f,Box::new(*v)))
                                       .collect();
        let res = {
            let select_field = self.filter_fields.iter()
                .skip(filter.len()).next().expect("missing filter field");
            self.words.complete_any(select_field, &filter)
        };
        self.count += 1;
        if self.count > 200 {
            error!("Aborting long phrase, possible loop, current filter values: {:?}", self.filter_values);
            return None;
        }
        match res {
            Ok(Some(n)) => {
                self.push(n);
                Some(n)
            },
            Ok(None) => None,
            Err(e) => {
                warn!("Ending early due to {:?}", e);
                None
            }
        }
    }
}

// join two vector phrases with spaces
pub fn join_phrase(phrase1: Vec<String>, phrase2: Vec<String>) -> String {
    phrase1.into_iter()
        .chain(phrase2)
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .as_slice()
        .join(" ")
}

// demote error result to Ok(None)
fn no_rows_as_none<T>(result: Result<Option<T>>) -> Result<Option<T>> {
    match result {
        Err(Error::QueryReturnedNoRows) => Ok(None),
        x => x
    }
}

// move errors to top, combine options
// (is there some kind of flat_map or collect that can do this?
fn into_result<T>(opt_result: Option<Result<Option<T>>>) -> Result<Option<T>> {
    match opt_result {
        None => Ok(None),
        Some(x) => x
    }
}

// reverse take n
fn last_n<T: Copy>(vec: &Vec<T>, limit: usize) -> Vec<T> {
    vec.iter().rev().take(limit).map(|x| *x).collect::<Vec<T>>()
        .into_iter().rev().collect::<Vec<T>>()
}

// split on white space and add begin/end sentinels
pub fn tokenize_phrase(phrase: &str) -> Vec<&str> {
    // vec![""].into_iter().chain(phrase.split_whitespace()).chain(vec![""]).collect::<Vec<&str>>()
    phrase.split_whitespace().collect::<Vec<&str>>()
}

// if needle is found, return a vec containing surrounding words
pub fn find_match_surround<'a>(needle: &str, haystack: &Vec<&'a str>) -> Option<Vec<&'a str>> {
    let lower_needle = needle.to_lowercase();
    // add begin/end framing
    let framed: Vec<&str> = vec![""].into_iter()
            .chain(haystack.iter().map(|x| *x))
            .chain(vec![""].into_iter())
            .collect();

    framed.iter()
        .position(|s| s.to_lowercase().starts_with(&lower_needle))
        .map(|pos| vec![
                // (does vec.get(-1) return None? does usize 0 - 1 panic?
                if pos > 0 { framed.get(pos-1)  } else { None },
                framed.get(pos),
                framed.get(pos+1)
            ].into_iter().flat_map(|o| o.map(|s| *s))
             .collect::<Vec<&str>>())
}

#[derive(Debug)]
pub struct WordsDb {
    db: Connection
}


impl WordsDb {
    pub fn new(db_url: String) -> WordsDb {
        debug!("Open db {}", db_url);
        WordsDb {
            db: Connection::open(db_url)
                .expect("Could not open database")
        }
    }
    pub fn from_config(optconfig: &Option<&Config>) -> WordsDb {
        let db_url: String = optconfig.as_ref()
            .and_then(|cfg| cfg.options.as_ref())
            .and_then(|opt| opt.get("words").and_then(|r| Some(r.clone())))
            .or_else(|| -> Option<String> { env::var("BAZBOT_WORDS").ok() } )
            .unwrap_or("bazbot.db".to_string());
        WordsDb::new(db_url)
    }
    pub fn new_from_env() -> WordsDb {
        let db_url = env::var("BAZBOT_WORDS")
            .expect("BAZBOT_WORDS must be set");
        WordsDb::new(db_url)
    }
    pub fn summary(&self) {
        println!("Summary of {:?}", self);
        let words = self.db.query_row(
            "select count(*) from words", &[], |row| row.get::<i64>(0));
        let phrases = self.db.query_row(
            "select count(*) from phrases", &[], |row| row.get::<i64>(0));
        match words.as_ref() {
            Ok(words) => println!("Words: {}", words),
            Err(e) => println!("Error counting words: {}", e)
        }
        match phrases.as_ref() {
            Ok(phrases) => println!("Phrases: {}", phrases),
            Err(e) => println!("Error counting phrases: {}", e)
        }
        if words.or(phrases).is_err(){
            println!("Migration may be necessary, is this a valid database?");
        }
    }

    pub fn migrate(&self) -> Result<()> {
        migration::migrate(&self.db)
    }

    fn complete_any(&self, select_field: &str,  filter: &Vec<NamedParam>) -> Result<Option<i64>> {
        match self.get_freq_where(&filter) {
            Ok(Some(freq)) => {
                let pick = random::<i64>().abs() % freq + 1;
                self.get_next_word_filter(select_field, &filter, pick)
            }
            result => result
        }
    }

    // collect a vector of ids, errors and nulls looking up words are ignored
    // This does not add new words so is only appropriate for feeding completion
    pub fn complete_id_vec(&self, prefix_words: &Vec<&str>) -> Vec<i64> {
        prefix_words.iter().flat_map(|pword| {
            let res = self.get_word_id(&pword);
            match res {
                Ok(Some(word_id)) => vec![ word_id ],
                Ok(None) => {
                    warn!("ignoring null value for {:?}", pword);
                    vec![]
                }
                Err(err) => {
                    error!("ignoring an error {:?}", err);
                    vec![]
                }
            }
        }).collect()
    }

    fn complete_ids<'a>(&'a self, filter1: WordField, filter2: WordField, filter3: WordField, filter_values: Vec<i64>) -> ChainIter {
        ChainIter {
            words: self,
            filter_fields: vec![filter1.to_str(), filter2.to_str(), filter3.to_str()],
            filter_values: filter_values,
            count: 0
        }
    }

    fn complete_forward<'a>(&'a self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::Word1, WordField::Word2, WordField::Word3, filter_values)
    }
    fn complete_backward<'a>(&'a self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::Word3, WordField::Word2, WordField::Word1, filter_values)
    }
    fn complete_middle<'a>(&'a self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::Word1, WordField::Word3, WordField::Word2, filter_values)
    }

    fn complete_and_map(&self, prefix: Vec<i64>) -> Result<Vec<String>> {
        // filter based on the last two words in prefix
        let filter = last_n(&prefix, 2);
        let words: Vec<Option<String>> =
            try!(prefix.into_iter()
                .chain(self.complete_forward(filter))
                .map(|id| self.get_spelling(id))
                .collect());
        return Ok(words.into_iter().flat_map(|x| x).collect())
    }

    pub fn complete_middle_out(&self, prefix: &Vec<&str> ) -> Result<Vec<String>> {
        debug!("complete middle out prefix: {:?}", prefix);
        let mut piter = prefix.iter();
        let first_word = try!(into_result(piter.next().map(|x| self.get_word_id(x))));
        let _ = piter.next();
        let last_word = try!(into_result(piter.next().map(|x| self.get_word_id(x))));
        let middle_filter: Vec<i64> = first_word.iter().chain(last_word.iter()).map(|i| *i).collect();
        let middle_word = if middle_filter.len() > 0 {
            // self.complete_middle(middle_filter).take(1).next()
            self.complete_middle(middle_filter).next()
        } else {
            None
        };
        if middle_word.is_some() {
            // filter is mid and at least one of first,last
            let filter: Vec<i64> = vec![first_word, middle_word, last_word].into_iter().flat_map(|x| x).collect();
            let back_filter: Vec<i64> = filter.clone().into_iter().take(2).collect::<Vec<i64>>().into_iter().rev().collect();
            let back_iter = self.complete_backward(back_filter);
            let back_words: Vec<i64> = back_iter.collect();
            let back_words: Vec<i64> = back_words.into_iter().rev().chain(filter).collect();
            self.complete_and_map(back_words)
        } else {
            debug!("No middle out match for {:?}, start new phrase", prefix);
            self.complete_and_map(vec![0])
        }
    }

    pub fn complete(&self, prefix: &Vec<&str> ) -> Result<Vec<String>> {
        let filter = self.complete_id_vec(&prefix);
        let words: Vec<Option<String>> =
            try!(self.complete_forward(filter)
                     .map(|id| self.get_spelling(id))
                     .collect());
        return Ok(words.into_iter().flat_map(|x| x).collect())
    }

    pub fn print_complete(&self, prefix: Vec<String> ) {
        let filter = if prefix.len() > 0 {
            let phrase = prefix.iter().map(AsRef::as_ref) .collect();
            find_match_surround("_", &phrase)
        } else {
            Some(vec![""])
        };

        match filter {
            Some(prefix) => {
                let result_words = self.complete_middle_out(&prefix);
                match result_words {
                    Ok(words) => println!("baz: {}", join_phrase(vec![], words)),
                    Err(e) => println!("Error: {:?}", e)
                };
            }
            None => println!("Couldn't find _ to complete against")
        };
    }

    pub fn read_file(&self, filename: String) -> Result<()> {
        let res = fs::File::open(&filename);
        let mut lines = 0;
        match res {
            Ok(file) => {
                debug!("file: {:?}", file);
                let tx = try!(self.db.transaction());
                let bufread = BufReader::new(&file);
                for line_res in bufread.lines() {
                    match line_res {
                        Ok(line) => {
                            try!(self.add_line(&line));
                            lines += 1;
                            if lines % 1000 == 0 {
                                debug!("Added {} lines", lines);
                            }
                        }
                        Err(e) => warn!("skipping: {:?}", e)
                    }
                }
                try!(tx.commit());
            }
            Err(err) => error!("err: {:?}", err)
        }
        info!("Added {} lines from {}", lines, filename);
        Ok(())
    }

    // add line as a phrase, assuming string separated by whitespace
    pub fn add_line(&self, line: &str) -> Result<()> {
        let words: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
        self.add_phrase(words)
    }

    pub fn add_phrase(&self, phrase: Vec<String> ) -> Result<()> {
        let v = try!(self.get_phrase_vec(phrase));
        let v1 = v.iter();
        let v2 = v.iter().skip(1);
        let v3 = v.iter().skip(2);
        for ((w1,w2),w3) in v1.zip(v2).zip(v3) {
            try!(self.increment_frequency(&[w1,w2,w3]));
        }
        Ok(())
    }

    fn increment_frequency(&self, words: &[&ToSql]) -> Result<i32> {
        let sql = "select 1 from phrases where word1=? and word2=? and word3=?;";
        let res = self.db.query_row(&sql, words,
            |row| row.get::<i64>(0));
        match res {
            Err(Error::QueryReturnedNoRows) => {
                let sql = "insert into phrases (freq, word1, word2, word3) values (1,?,?,?);";
                self.db.execute(sql, words)
            },
            Ok(_) => {
                let sql = "update phrases set freq=freq+1 where word1=? and word2=? and word3=?;";
                self.db.execute(sql, words)
            },
            Err(e) => Err(e)
        }
    }

    // lookup word ids and surround with begin/end 0s
    fn get_phrase_vec(&self, phrase: Vec<String>) -> Result<Vec<i64>> {
        let result: Vec<i64> = try!(phrase.iter().map(
            |w| self.get_or_add_word_id(w))
            .collect());
        Ok(vec![0].into_iter().chain(result.into_iter()).chain(vec![0].into_iter()).collect())
    }

    fn get_freq_where(&self, filter: &Vec<NamedParam>) -> Result<Option<i64>> {
        let wheres = NamedParam::assigns(filter);
        let values = NamedParam::values(filter);
        let sql_where = if wheres.as_slice().is_empty() {
            String::from("")
        } else {
            format!("where {}", wheres.join(&String::from(" and ")))
        };
        let sql = format!("select sum(freq) from phrases {}", sql_where);

        no_rows_as_none(self.db.query_row(&sql, values.as_slice(),
            |row| row.get::<Option<i64>>(0)))
    }

    fn get_next_word_filter(&self, select_field: &str, prefix_filter: &Vec<NamedParam>, pick: i64)
        -> Result<Option<i64>> {
        let wheres = NamedParam::assigns(prefix_filter);
        let values = NamedParam::values(prefix_filter);
        // retrieve column based on how many words in prefix
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

    fn get_or_add_word_id(&self, spelling: &str) -> Result<i64> {
        let res = self.get_word_id(spelling);
        match res {
            Ok(None) => {
                try!(self.db.execute("insert into words (spelling) values (?)", &[&spelling]));
                Ok(self.db.last_insert_rowid())
            },
            Ok(Some(word_id)) => Ok(word_id),
            Err(e) => Err(e)
        }
    }

    fn get_word_id(&self, spelling: &str) -> Result<Option<i64>> {
        no_rows_as_none(self.db.query_row(
            "select word_id from words where spelling=?",
            &[&spelling], |row| row.get::<Option<i64>>(0)))
    }

    fn get_spelling(&self, word_id: i64) -> Result<Option<String>> {
        no_rows_as_none(self.db.query_row(
            "select spelling from words where word_id=?",
            &[&word_id], |row| row.get::<Option<String>>(0)))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn memdb() -> WordsDb {
        WordsDb::new(":memory:".to_string())
    }
    fn abcde() -> WordsDb {
        let w = memdb();
        w.migrate().expect("migrate");
        w.add_line("a b c d e").expect("read line");
        w
    }
    
    #[test]
    fn summary() {
        let c = abcde();
        c.summary()
    }
    fn assert_next(words: &WordsDb, chain: &mut ChainIter, expected: &str) {
        let got = chain.next().map(|id| words.get_spelling(id));
        assert_eq!(got.unwrap().unwrap().unwrap(), expected);
    }
    #[test]
    fn forward1() {
        let w = abcde();
        let filter = w.complete_id_vec(&vec![""]);
        let mut chain = w.complete_forward(filter);
        assert_next(&w, &mut chain, "a");
        assert_next(&w, &mut chain, "b");
    }
    #[test]
    fn forward2() {
        let w = abcde();
        let filter = w.complete_id_vec(&vec!["","a"]);
        let mut chain = w.complete_forward(filter);
        assert_next(&w, &mut chain, "b");
        assert_next(&w, &mut chain, "c");
    }
    #[test]
    fn backward() {
        let w = abcde();
        let filter = w.complete_id_vec(&vec!["","e"]);
        let mut chain = w.complete_backward(filter);
        assert_next(&w, &mut chain, "d");
        assert_next(&w, &mut chain, "c");
    }
    #[test]
    fn middle1() {
        let w = abcde();
        let filter = w.complete_id_vec(&vec!["","b"]);
        let mut chain = w.complete_middle(filter);
        assert_next(&w, &mut chain, "a");
    }
    #[test]
    fn middle2() {
        let w = abcde();
        let filter = w.complete_id_vec(&vec!["b","d"]);
        let mut chain = w.complete_middle(filter);
        assert_next(&w, &mut chain, "c");
        let none = chain.next();
        assert_eq!(None, none);
    }
    #[test]
    fn complete_and_map() {
        let w = abcde();
        let complete: Vec<String> = w.complete_and_map(vec![0]).unwrap();
        assert_eq!(vec!["","a","b","c","d","e",""], complete);
    }
    #[test]
    fn search_middle() {
        let filter = find_match_surround("baz", &tokenize_phrase("a b baz d e")).unwrap();
        assert_eq!(vec!["b","baz","d"], filter);
    }
    #[test]
    fn search_begin() {
        let filter = find_match_surround("baz", &tokenize_phrase("Baz, b c d e")).unwrap();
        assert_eq!(vec!["","Baz,","b"], filter);
    }
    #[test]
    fn search_end() {
        let filter = find_match_surround("baz", &tokenize_phrase("a b c d BAZOO!")).unwrap();
        assert_eq!(vec!["d","BAZOO!",""], filter);
    }
    #[test]
    fn search_min() {
        let filter = find_match_surround("baz", &tokenize_phrase("baz")).unwrap();
        assert_eq!(vec!["","baz",""], filter);
    }
}
