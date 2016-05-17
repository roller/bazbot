extern crate baz;
extern crate clap;
extern crate dotenv;
use clap::{App, Arg, SubCommand, ArgMatches};
use baz::markov_words::WordsDb;
use baz::ircconn::IrcConn;

fn cmd_complete(baz: &WordsDb, matches: &ArgMatches) {
    let prefix = matches.values_of_lossy("prefix").unwrap_or(vec![]);
    println!("The prefix args: {:?}", prefix);
    baz.print_complete(prefix);
}

fn cmd_migrate(baz: &WordsDb, _matches: &ArgMatches) {
     let res = baz.migrate();
     println!("Migrate: {:?}", res);
 }

fn cmd_irc(baz: WordsDb, matches: &ArgMatches) {
    let opt_config = matches.value_of_lossy("config");
    let irc = match opt_config {
        Some(config) => IrcConn::new_from_config(baz, &config),
        None => IrcConn::new_from_env(baz)
    };
    irc.run()
}

fn main(){
    dotenv::dotenv().ok();
    let bazargs = App::new("BenzoBaz WordBot")
        .version("0.1.1")
        .author("Joel Roller <roller@gmail.com>")
        .subcommand(SubCommand::with_name("summary")
            .about("Summarize database"))
        .subcommand(SubCommand::with_name("migrate")
            .about("Create or sync database against current code"))
        .subcommand(SubCommand::with_name("complete")
            .about("Run a markov chain starting with args")
            .arg(Arg::with_name("prefix").multiple(true)))
        .subcommand(SubCommand::with_name("irc")
            .about("Connect to irc server")
            .arg(Arg::with_name("config")))
        .after_help("\nReads configuration from environment or .env file:\n\
                     WORDS_DB=db/words.db    Location of sqlite words db")
        .get_matches();

    let baz = WordsDb::new_from_env();

    match bazargs.subcommand() {
        ("summary", Some(_)) => baz.summary(),
        ("migrate", Some(subm)) => cmd_migrate(&baz, subm),
        ("complete", Some(subm)) => cmd_complete(&baz, subm),
        ("irc", Some(subm)) => cmd_irc(baz, subm),
        _ => {
            // Can't use App print_help because we
            // used get_matches instead.
            println!("Unknown subcommand (try help)");
        }
    }
}
