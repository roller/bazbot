use clap::{App, Arg, SubCommand, ArgMatches, AppSettings};
use bazbot::markov_words::WordsDb;
use bazbot::ircconn::IrcConn;
use irc::client::data::config::Config;
use std::env;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

fn cmd_add_phrase(words: &WordsDb, matches: &ArgMatches) {
    let phrase = matches.values_of_lossy("words").unwrap_or_default();
    words.add_phrase(&phrase).expect("failed");
}

fn cmd_read_phrases(words: &mut WordsDb, matches: &ArgMatches) {
    let files = matches.values_of_lossy("files").unwrap_or_default();
    for file in files {
        words.read_file(&file).expect("couldn't read file");
    }
}

fn cmd_complete(words: &WordsDb, matches: &ArgMatches) {
    let prefix = matches.values_of_lossy("prefix").unwrap_or_default();
    words.print_complete(&prefix);
}

async fn cmd_irc(words: WordsDb, config: Config) {
    let mut irc = IrcConn::new_from_config(words, config);
    irc.run().await
}

#[tokio::main]
async fn main(){
    dotenv::dotenv().ok();
    // log info level by default
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let bazargs = App::new("Bazbot Blabberbot")
        .version(VERSION.unwrap_or("v0"))
        .arg(Arg::with_name("config")
            .short("c").long("config")
            .takes_value(true)
            .value_name("FILE.toml")
            .required(false)
            .help("Config file (defaults to env var BAZBOT_CONFIG or bazbot.toml)."))
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("summary")
            .about("Summarize database"))
        .subcommand(SubCommand::with_name("complete")
            .about("Run a markov chain matching args around _")
            .arg(Arg::with_name("prefix").multiple(true)))
        .subcommand(SubCommand::with_name("add")
            .about("Add a phrase to the markov words database")
            .arg(Arg::with_name("words").multiple(true)))
        .subcommand(SubCommand::with_name("read")
            .about("Read text file with one phrase per line into markov database")
            .arg(Arg::with_name("files").multiple(true).value_name("file.txt")))
        .subcommand(SubCommand::with_name("irc")
            .about("Interact on irc channels"))
        .after_help("
Files:
    - bazbot.toml or
    - bazbot.json
        A valid config file is required to connect to irc.
        See irc library documentation for more information:
           https://github.com/aatxe/irc
        The following options are supported:
         - words - sqlite database to store phrases
         - learn - learn new phrases from irc
    - bazbot.db
        default sqlite file storing phrases

Environment
    Additional environment will be read from .env
    RUST_LOG        - debug, info, warn, error (default info)
    BAZBOT_CONFIG   - default toml or json config file location
    BAZBOT_WORDS    - default sqlite database location")
        .get_matches();

    let cfg_file: String = bazargs.value_of_lossy("config")
        .map(|arg| arg.to_string())
        .or_else(|| env::var("BAZBOT_CONFIG").ok())
        .unwrap_or_else(|| "bazbot.toml".to_string());
    let cfg = Config::load(&cfg_file).expect(&format!("Couldn't load config file {}", &cfg_file));
    let mut words = WordsDb::from_config(&cfg);
    words.migrate().expect("Database migration failed");

    match bazargs.subcommand() {
        ("summary", Some(_)) => words.summary(),
        ("add", Some(subm)) => cmd_add_phrase(&words, subm),
        ("read", Some(subm)) => cmd_read_phrases(&mut words, subm),
        ("complete", Some(subm)) => cmd_complete(&words, subm),
        ("irc", Some(_)) => cmd_irc(words, cfg).await,
        _ => {
            // Can't use App print_help because we
            // used get_matches instead.
            println!("Unknown subcommand (try help)");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn we_dont_know_what_to_test_in_main() {
        assert!(true);
    }
}
