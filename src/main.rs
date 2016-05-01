extern crate baz;
extern crate clap;
extern crate dotenv;
use clap::{Arg, App, SubCommand};


fn main(){
    dotenv::dotenv().ok();
    let bazargs = App::new("BenzoBaz WordBot")
        .version("0.1")
        .author("Joel Roller <roller@gmail.com>")
        .subcommand(SubCommand::with_name("summary")
            .about("Summarize database"))
        .after_help("\nReads configuration from environment or .env file:\n\
                     WORDS_DB=db/words.db    Location of sqlite words db")
        .get_matches();

    let baz = baz::Baz::new();

    match bazargs.subcommand_name() {
        Some("summary") => baz.summary(),
        _ => {
            // Can't use App print_help because we
            // used get_matches instead.
            println!("Unknown subcommand (try help)");
        }
    }
}
