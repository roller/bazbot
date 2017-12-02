extern crate bazbot;
extern crate clap;
extern crate dotenv;
extern crate env_logger;
extern crate irc;
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

fn cmd_irc(words: WordsDb, config: Config) {
    let irc = IrcConn::new_from_config(words, config);
    irc.run()
}

fn main(){
    dotenv::dotenv().ok();
    // log info level by default
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init().unwrap();

    let bazargs = App::new("Bazbot Blabberbot")
        .version(VERSION.unwrap_or("v0"))
        .arg(Arg::with_name("config")
            .short("c").long("config")
            .takes_value(true)
            .value_name("FILE.json")
            .required(false)
            .help("Read config from json file (defaults to env var BAZBOT_CONFIG or bazbot.json)."))
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
    bazbot.json
        A valid config file is required to connect to irc.
        Words db location may be configured as options.words, eg:
            { \"options\": { \"words\": \"bazbot.db\" } }
        See irc library documentation for more information:
           https://github.com/aatxe/irc
    bazbot.db
        sqlite file with bazbot's brain

Environment
    Additional environment will be read from .env
    RUST_LOG        - debug, info, warn, error (default info)
    BAZBOT_CONFIG   - default json config file location
    BAZBOT_WORDS    - default sqlite database location")
        .get_matches();

    let cfg_file: String = bazargs.value_of_lossy("config")
        .and_then(|arg| Some(arg.to_string()))
        .or_else(|| env::var("BAZBOT_CONFIG").ok())
        .unwrap_or_else(|| "bazbot.json".to_string());
    // let cfg = Config::load(&cfg_file).expect(&format!("Couldn't load config file {}", &cfg_file));
    let cfg = Config::load(&cfg_file);
    let mut words = WordsDb::from_config(&cfg.as_ref().ok());
    words.migrate().expect("Database migration failed");

    match bazargs.subcommand() {
        ("summary", Some(_)) => words.summary(),
        ("add", Some(subm)) => cmd_add_phrase(&words, subm),
        ("read", Some(subm)) => cmd_read_phrases(&mut words, subm),
        ("complete", Some(subm)) => cmd_complete(&words, subm),
        ("irc", Some(_)) => cmd_irc(words, cfg.expect(&format!("Couldn't load config file {}", &cfg_file))),
        _ => {
            // Can't use App print_help because we
            // used get_matches instead.
            println!("Unknown subcommand (try help)");
        }
    }
}
