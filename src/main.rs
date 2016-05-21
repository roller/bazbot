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

fn cmd_complete(words: &WordsDb, matches: &ArgMatches) {
    let prefix = matches.values_of_lossy("prefix").unwrap_or(vec![]);
    println!("Prefix: {:?}", prefix);
    words.print_complete(prefix);
}

fn cmd_migrate(words: &WordsDb, _matches: &ArgMatches) {
     let res = words.migrate();
     println!("Migrate: {:?}", res);
 }

fn cmd_irc(words: WordsDb, config: Config) {
    let irc = IrcConn::new_from_config(words, config);
    irc.run()
}

fn main(){
    dotenv::dotenv().ok();
    env_logger::init().unwrap();

    let bazargs = App::new("Bazbot Blabberbot")
        .version("0.2.0")
        .author("Joel Roller <roller@gmail.com>")
        .arg(Arg::with_name("config")
            .short("c").long("config")
            .takes_value(true)
            .value_name("FILE.json")
            .required(false)
            .help("Read config from json file (defaults to env var BAZBOT_CONFIG or bazbot.config)."))
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("summary")
            .about("Summarize database"))
        .subcommand(SubCommand::with_name("migrate")
            .about("Create or sync database against current code"))
        .subcommand(SubCommand::with_name("complete")
            .about("Run a markov chain starting with args")
            .arg(Arg::with_name("prefix").multiple(true)))
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
    RUST_LOG        - debug, info, warn, error
    BAZBOT_CONFIG   - default json config file location
    BAZBOT_WORDS    - default sqlite database location")
        .get_matches();

    let cfg_file: String = bazargs.value_of_lossy("config")
        .and_then(|arg| Some(arg.to_string()))
        .or_else(|| env::var("BAZBOT_CONFIG").ok())
        .unwrap_or("bazbot.config".to_string());
    // let cfg = Config::load(&cfg_file).expect(&format!("Couldn't load config file {}", &cfg_file));
    let cfg = Config::load(&cfg_file);
    let words = WordsDb::from_config(&cfg.as_ref().ok());

    match bazargs.subcommand() {
        ("summary", Some(_)) => words.summary(),
        ("migrate", Some(subm)) => cmd_migrate(&words, subm),
        ("complete", Some(subm)) => cmd_complete(&words, subm),
        ("irc", Some(_)) => cmd_irc(words, cfg.expect(&format!("Couldn't load config file {}", &cfg_file))),
        _ => {
            // Can't use App print_help because we
            // used get_matches instead.
            println!("Unknown subcommand (try help)");
        }
    }
}
