-- update database from old baz database to match current application
-- update words database to remove nulls
begin;

create table migrations ( m_id primary key );
insert into migrations values ("init");

create unique index idx_spelling on words (spelling);

CREATE TABLE phrases_no_null (
    word1 integer not null,
    word2 integer not null,
    word3 integer not null,
    freq integer
);
CREATE UNIQUE INDEX idx_phrases_u on phrases_no_null (word1,word2,word3);

insert into phrases_no_null 
select word1, word2, word3, freq from phrases
where word1 is not null and word2 is not null and word3 is not null;

-- zero word1
insert into phrases_no_null
select 0 as word1, word2, word3, sum(freq) as freq from phrases
where word1 is null and word2 is not null and word3 is not null
group by word2, word3;

-- zero word2 (none expected)
insert into phrases_no_null
select word1, 0 as word2, word3, sum(freq) as freq from phrases
where word1 is not null and word2 is null and word3 is not null
group by word1, word3;

-- zero word3
insert into phrases_no_null
select word1, word2, 0 as word3, sum(freq) as freq from phrases
where word1 is not null and word2 is not null and word3 is null
group by word1, word2;


drop table phrases;
alter table phrases_no_null rename to phrases;
insert into migrations values ("words_and_phrases_init");

commit;
vacuum;
