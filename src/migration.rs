
use rusqlite::{Result, Connection,Error};
use rusqlite::types::ToSql;

#[derive(Debug)]
pub struct Migration {
    pub m_id: &'static str,
    pub m_sql: &'static str,
}

pub fn base() -> Migration {
    Migration {
        m_id: "init",
        m_sql: "create table migrations ( m_id primary key );"
    }
}

pub fn migrations() -> Vec<Migration> {
    vec![
    Migration {
        m_id: "words_and_phrases_init",
        m_sql: "
        CREATE TABLE words (word_id integer primary key autoincrement, spelling text not null);
        CREATE TABLE phrases (
            word1 integer not null, word2 integer not null, word3 integer not null, freq integer not null,
            foreign key (word1) references words(word_id),
            foreign key (word2) references words(word_id),
            foreign key (word3) references words(word_id)
        );
        insert into words (word_id, spelling) values (0,'');
        CREATE UNIQUE INDEX idx_words on words (word_id);
        CREATE UNIQUE INDEX idx_spelling on words (spelling);
        CREATE UNIQUE INDEX idx_phrases_u on phrases (word1,word2,word3);"
    },
    Migration {
        m_id: "phrases_spelling_view",
        m_sql: "
        create view phrases_spelling as
        select w1.spelling as word1, w2.spelling as word2, w3.spelling as word3, freq
        from phrases
        inner join words w1 on phrases.word1 = w1.word_id
        inner join words w2 on phrases.word2 = w2.word_id
        inner join words w3 on phrases.word3 = w3.word_id;
        "
    }]
}

pub fn migrate<'a>(db: &'a Connection) -> Result<()> {
    let m = Migrator::new(db);
    m.migrate()
}


pub struct Migrator<'a> {
    db: &'a Connection
}

impl<'a> Migrator<'a> {
    fn new(db: &'a Connection) -> Migrator<'a> {
        Migrator { db: db }
    }

    fn migrate(&self) -> Result<()> {
        try!(self.base_migration());
        for migration in migrations() {
            if !try!(self.check_migration(&migration)) {
                try!(self.run_migration(&migration))
            }
        }
        Ok(())
    }

    // Use a different technique for the very first migration:
    // check sqlite_master table for the existance of the table
    fn base_migration(&self) -> Result<()> {
        let sql = "select name from sqlite_master where type='table' and name='migrations';";
        let res = self.db.query_row(sql, &[], |_| ());
        match res {
            Ok(()) => Ok(()),
            Err(Error::QueryReturnedNoRows) => self.run_migration(&base()),
            e @ Err(_) => e
        }
    }

    // returt true if migration is already logged in db
    fn check_migration(&self, migration: &Migration) -> Result<bool> {
        let check_sql = "select 1 from migrations where m_id = ?";
        let params : Vec<&ToSql> = vec![&migration.m_id];
        let res = self.db.query_row(&check_sql, &params,
            |row| row.get::<Option<i64>>(0));
        match res {
            Err(Error::SqliteFailure(_,_)) => Ok(false), // returned when no migration table
            Err(Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(e),
            Ok(_) => Ok(true)
        }
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
}

