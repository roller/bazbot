extern crate irc;

use migration;
use std::{env,fs};
use std::io::{BufRead,BufReader};
use rusqlite::{Result, Connection,Error};
use rusqlite::types::ToSql;
use rand::random;
use self::irc::client::data::config::Config;

enum WordField {
    One,
    Two,
    Three,
}

impl WordField {
    fn into_str(self) -> &'static str {
        self.to_str()
    }
    fn to_str(&self) -> &'static str {
        match *self {
            WordField::One => "word1",
            WordField::Two => "word2",
            WordField::Three => "word3"
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
            value
        }
    }
    fn assigns(params: &[NamedParam]) -> Vec<String> {
        params.iter().map(|w| { format!("{}=?", w.field) }).collect()
    }
    fn values(params: &'a [NamedParam]) -> Vec<&'a ToSql> {
        params.iter().map(|w| &*w.value).collect()
    }
}

pub struct ChainIter<'a> {
    words: &'a WordsDb,
    // database fields to filter or select:
    // -  where there's a corresponding value, this is used
    //    to filter the next item
    // - the first field after the value is the output field
    filter_fields: Vec<&'a str>,  // Should always be 3
    filter_values: Vec<i64>,      // 0-2 values
    // infinite loop guard in case of bad data
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
            let select_field = self.filter_fields
                .get(filter.len()).expect("missing filter field");
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

// demote error result to Ok(None) and combine results
fn no_rows_as_none<T>(result: Result<Result<Option<T>>>) -> Result<Option<T>> {
    match result {
        Err(Error::QueryReturnedNoRows) => Ok(None),
        Err(x) | Ok(Err(x)) => Err(x),
        Ok(ok) => ok
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
fn last_n<T: Copy>(vec: &[T], limit: usize) -> Vec<T> {
    vec.iter().rev().take(limit).cloned().collect::<Vec<T>>()
        .into_iter().rev().collect::<Vec<T>>()
}

// split on white space and add begin/end sentinels
pub fn tokenize_phrase(phrase: &str) -> Vec<&str> {
    // vec![""].into_iter().chain(phrase.split_whitespace()).chain(vec![""]).collect::<Vec<&str>>()
    phrase.split_whitespace().collect::<Vec<&str>>()
}

// Search haystack for nearby phrases:
// returns vec of:
//  length 0 if needle was not found
//  length 1 or 2 if interesting nearby words found
// each vec is a vec of 2 words or ""
pub fn find_nearby<'a>(needle: &str, haystack: &[&'a str]) -> Vec<Vec<&'a str>> {
    let lower_needle = needle.to_lowercase();
    // add begin/end framing
    let framed: Vec<&str> = vec![""].into_iter()
            .chain(haystack.iter().cloned())
            .chain(vec![""].into_iter())
            .collect();

    if let Some(pos) = framed.iter()
        .position(|s| s.to_lowercase().starts_with(&lower_needle)) {
        let mut found = vec![];
        if pos > 1 {
            found.push( vec![ framed[pos-2], framed[pos-1] ] );
        }

        if pos < framed.len() - 2 {
            found.push( vec![ framed[pos+1], framed[pos+2] ] );
        }

        if found.is_empty() {
            found.push( vec![""] );
        }
        found

    } else {
        vec![]
    }
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
    pub fn from_config(optconfig: Option<&Config>) -> WordsDb {
        let db_url: String = optconfig.as_ref()
            .and_then(|cfg| cfg.options.as_ref())
            .and_then(|opt| opt.get("words").and_then(|r| Some(r.clone())))
            .or_else(|| -> Option<String> { env::var("BAZBOT_WORDS").ok() } )
            .unwrap_or_else(|| "bazbot.db".to_string());
        WordsDb::new(db_url)
    }
    pub fn new_from_env() -> WordsDb {
        let db_url = env::var("BAZBOT_WORDS")
            .expect("BAZBOT_WORDS must be set");
        WordsDb::new(db_url)
    }
    pub fn summary(&self) {
        println!("Summary of {:?}", self);
        let words: Result<i64> = self.db.query_row(
            "select count(*) from words", &[], |row| row.get(0));
        match words.as_ref() {
            Ok(words) => println!("Words: {}", words),
            Err(e) => println!("Error counting words: {}", e)
        }
        let phrases = self.db.query_row(
            "select count(*) from phrases", &[], |row| row.get(0));
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

    fn complete_any(&self, select_field: &str,  filter: &[NamedParam]) -> Result<Option<i64>> {
        match self.get_freq_where(filter) {
            Ok(Some(freq)) => {
                let pick = random::<i64>().abs() % freq + 1;
                self.get_next_word_filter(select_field, filter, pick)
            }
            result => result
        }
    }

    // collect a vector of ids, errors and nulls looking up words are ignored
    // This does not add new words so is only appropriate for feeding completion
    pub fn complete_id_vec(&self, prefix_words: &[&str]) -> Vec<i64> {
        prefix_words.iter().flat_map(|pword| {
            let res = self.get_word_id(pword);
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

    fn complete_ids(&self, filter1: WordField, filter2: WordField, filter3: WordField, filter_values: Vec<i64>) -> ChainIter {
        ChainIter {
            words: self,
            filter_fields: vec![filter1.into_str(), filter2.into_str(), filter3.into_str()],
            filter_values,
            count: 0
        }
    }

    fn complete_forward(&self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::One, WordField::Two, WordField::Three, filter_values)
    }
    fn complete_backward(&self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::Three, WordField::Two, WordField::One, filter_values)
    }
    // middle is intended for single lookup to prime other completions
    // for example:  1 2 3
    // Looking up (2) based on 1 and 3 isn't something that can reasonably chain further
    fn complete_middle(&self, filter_values: Vec<i64>) -> ChainIter {
        self.complete_ids(WordField::One, WordField::Three, WordField::Two, filter_values)
    }

    fn complete_and_map(&self, prefix: Vec<i64>) -> Result<Vec<String>> {
        // filter based on the last two words in prefix
        let filter = last_n(&prefix, 2);
        let words: Vec<Option<String>> =
            try!(prefix.into_iter()
                .chain(self.complete_forward(filter))
                .map(|id| self.get_spelling(id))
                .collect());
        Ok(words.into_iter().flat_map(|x| x).collect())
    }

    fn count_nearby(&self, w1: i64, w2: i64) -> Result<i64> {
        let sql = "select coalesce(sum(freq),0) from phrases where (word1=? and word2=?) or (word2=? and word3=?)";
        let res = self.db.query_row(sql, &[&w1,&w2,&w1,&w2], |row| row.get(0));
        match res {
            Err(Error::QueryReturnedNoRows) => Ok(0),
            Ok(count) => Ok(count),
            Err(e) => Err(e)
        }
    }

    fn prime_from_nearby(&self, prefixes: Vec<Vec<&str>>) -> Result<Vec<i64>> {
        let mut prefix_ids: Vec<Vec<i64>> = Vec::with_capacity(prefixes.len());
        let mut prefix_counts: Vec<i64> = Vec::with_capacity(prefixes.len());
        let mut total_count = 0;
        for prefix in prefixes {
            let ids = self.complete_id_vec(prefix.as_slice());
            let count = match ids.len() {
                // The goal is to match pairs
                2 => try!(self.count_nearby(ids[0],ids[1])),
                // don't count, but give a chance for being used
                1 => 1,
                // I don't even
                _ => 0
            };
            if count > 0 {
                prefix_ids.push(ids);
                prefix_counts.push(count);
                total_count += count;
            }
        }
        if total_count > 0 {
            // chose a phrase to prime
            let mut pick = random::<i64>().abs() % total_count + 1;
            for (count, prefix) in prefix_counts.into_iter().zip(prefix_ids) {
                pick -= count;
                if pick <= 0 {
                    return Ok(prefix)
                }
            }
        }
        // initialize from single stop token
        Ok(vec![0])
    }

    /// The goal is to query nearby words to initialize phrases
    ///
    /// Start with an example phrase: X X A B _ D E X X
    /// (where _ is the matched word)
    /// markov the choice AB vs DE via SQL:
    /// ``` SQL
    /// select 0, sum(freq) from phrases_spelling
    /// where (word1='A' and word2='B') or (word2='A' and word3='B')
    /// union all
    /// select 1, sum(freq) from phrases_spelling
    /// where (word1='D' and word2='E') or (word2='D' and word3='E')
    /// ```
    /// This result can then be used to initialize forward and backward chains
    /// challenge:
    ///  - A and E are optional (which would result in different query), but only when:
    ///  - B and D may be stop tokens, that's fine, but we probably don't want
    ///    BOTH B and D to be stop tokens.  Initializing on only stop token
    ///    is considered uninteresting, but may be the only choice

    pub fn new_complete_middle_out(&self, prefixes: Vec<Vec<&str>>) -> Result<Vec<String>> {
        let primer = try!(self.prime_from_nearby(prefixes));
        let words = if primer[0] == 0 {
            primer
        } else {
            let back_filter: Vec<i64> = primer.clone().into_iter().rev().collect();
            let back_iter = self.complete_backward(back_filter);
            let back_words: Vec<i64> = back_iter.collect();
            back_words.into_iter().rev().chain(primer).collect()
        };
        self.complete_and_map(words)
    }

    pub fn complete_middle_out(&self, prefix: &[&str] ) -> Result<Vec<String>> {
        debug!("complete middle out prefix: {:?}", prefix);
        let mut piter = prefix.iter();
        let first_word = try!(into_result(piter.next().map(|x| self.get_word_id(x))));
        let _ = piter.next();
        let last_word = try!(into_result(piter.next().map(|x| self.get_word_id(x))));
        let middle_filter: Vec<i64> = first_word.iter().chain(last_word.iter()).cloned().collect();
        let middle_word = if middle_filter.is_empty() {
            None
        } else {
            self.complete_middle(middle_filter).next()
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

    pub fn complete(&self, prefix: &[&str] ) -> Result<Vec<String>> {
        let filter = self.complete_id_vec(prefix);
        let words: Vec<Option<String>> =
            try!(self.complete_forward(filter)
                     .map(|id| self.get_spelling(id))
                     .collect());
        Ok(words.into_iter().flat_map(|x| x).collect())
    }

    pub fn print_complete(&self, prefix: &[String] ) {
        let filter = if prefix.is_empty() {
            // no prefix, initialize from an end-of-phrase sentinel value,
            vec![vec![""]]
        } else {
            let phrase: Vec<&str> = prefix.iter().map(AsRef::as_ref) .collect();
            find_nearby("_", phrase.as_slice())
        };

        match filter.len() {
            0 => println!("Couldn't find _ to complete against"),
            _ => {
                let result_words = self.new_complete_middle_out(filter);
                match result_words {
                    Ok(words) => println!("{}", join_phrase(vec![], words)),
                    Err(e) => println!("Error: {:?}", e)
                };
            }
        };
    }

    pub fn read_file(&mut self, filename: &str) -> Result<()> {
        let res = fs::File::open(&filename);
        let mut lines = 0;
        match res {
            Ok(file) => {
                debug!("file: {:?}", file);
                let tx = self.db.transaction()?;
                let bufread = BufReader::new(&file);
                for line_res in bufread.lines() {
                    match line_res {
                        Ok(line) => {
                            // try to run this pattern in a test
                            Self::add_line_db(&tx, &line)?;
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
        Self::add_line_db(&self.db, line)
    }
    fn add_line_db(db: &Connection, line: &str) -> Result<()> {
        let words: Vec<String> = line.split_whitespace().map(ToString::to_string).collect();
        Self::add_phrase_db(db, &words)
    }

    pub fn add_phrase(&self, phrase: &[String] ) -> Result<()> {
        Self::add_phrase_db(&self.db, phrase)
    }
    fn add_phrase_db(db: &Connection, phrase: &[String] ) -> Result<()> {
        let v = try!(Self::get_phrase_vec(db, phrase));
        let v1 = v.iter();
        let v2 = v.iter().skip(1);
        let v3 = v.iter().skip(2);
        for ((w1,w2),w3) in v1.zip(v2).zip(v3) {
            try!(Self::increment_frequency_db(db, &[w1,w2,w3]));
        }
        Ok(())
    }

    fn increment_frequency_db(db: &Connection, words: &[&ToSql]) -> Result<usize> {
        let sql = "select 1 from phrases where word1=? and word2=? and word3=?;";
        let res: Result<i64> = db.query_row(sql, words, |row| row.get(0));
        match res {
            Err(Error::QueryReturnedNoRows) => {
                let sql = "insert into phrases (freq, word1, word2, word3) values (1,?,?,?);";
                db.execute(sql, words)
            },
            Ok(_) => {
                let sql = "update phrases set freq=freq+1 where word1=? and word2=? and word3=?;";
                db.execute(sql, words)
            },
            Err(e) => Err(e)
        }
    }

    // lookup word ids and surround with begin/end 0s
    fn get_phrase_vec(db: &Connection, phrase: &[String]) -> Result<Vec<i64>> {
        let result: Vec<i64> = try!(phrase.iter().map(
            |w| Self::get_or_add_word_id(db, w))
            .collect());
        Ok(vec![0].into_iter().chain(result.into_iter()).chain(vec![0].into_iter()).collect())
    }

    fn get_freq_where(&self, filter: &[NamedParam]) -> Result<Option<i64>> {
        let wheres = NamedParam::assigns(filter);
        let values = NamedParam::values(filter);
        let sql_where = if wheres.as_slice().is_empty() {
            String::from("")
        } else {
            format!("where {}", wheres.join(&String::from(" and ")))
        };
        let sql = format!("select sum(freq) from phrases {}", sql_where);

        no_rows_as_none(self.db.query_row(&sql, values.as_slice(),
            |row| row.get_checked(0)))
    }

    fn get_next_word_filter(&self, select_field: &str, prefix_filter: &[NamedParam], pick: i64)
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

        let mut rows = try!(stmt.query(&values));
        // for result_row in rows {
        while let Some(result_row) = rows.next() {
            let row = try!(result_row);
            let freq: i64 = row.get(0);
            if pick_count <= freq {
                return Ok(row.get(1));
            }
            pick_count -= freq;
        }
        Ok(None)
    }

    fn get_or_add_word_id(db: &Connection, spelling: &str) -> Result<i64> {
        let res = Self::get_word_id_db(db, spelling);
        match res {
            Ok(None) => {
                try!(db.execute("insert into words (spelling) values (?)", &[&spelling]));
                Ok(db.last_insert_rowid())
            },
            Ok(Some(word_id)) => Ok(word_id),
            Err(e) => Err(e)
        }
    }

    fn get_word_id(&self, spelling: &str) -> Result<Option<i64>> {
        Self::get_word_id_db(&self.db, spelling)
    }
    fn get_word_id_db(db: &Connection, spelling: &str) -> Result<Option<i64>> {
        no_rows_as_none(db.query_row(
            "select word_id from words where spelling=?",
            &[&spelling], |row| row.get_checked(0)))
    }

    fn get_spelling(&self, word_id: i64) -> Result<Option<String>> {
        no_rows_as_none(self.db.query_row(
            "select spelling from words where word_id=?",
            &[&word_id], |row| row.get_checked(0)))
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
    fn forward() {
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
    fn middle() {
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
        let filter = find_nearby("baz", &tokenize_phrase("a b baz d e"));
        assert_eq!(vec![vec!["a","b"], vec!["d","e"]], filter);
    }
    #[test]
    fn search_begin() {
        let filter = find_nearby("baz", &tokenize_phrase("Baz, b c d e"));
        assert_eq!(vec![vec!["b","c"]], filter);
    }
    #[test]
    fn search_end() {
        let filter = find_nearby("baz", &tokenize_phrase("a b c d BAZOO!"));
        assert_eq!(vec![vec!["c","d"]], filter);
    }
    #[test]
    fn search_min() {
        let filter = find_nearby("baz", &tokenize_phrase("baz"));
        assert_eq!(vec![vec![""]], filter);
    }
    #[test]
    fn search_fail() {
        let filter = find_nearby("baz", &tokenize_phrase("That's not a knife!"));
        let empty: Vec<Vec<&str>> = vec![];
        assert_eq!(empty, filter);
    }


}
