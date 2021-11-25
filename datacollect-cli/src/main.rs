pub(crate) mod common;
mod modules;
mod options;

use std::io::stdout;

use erased_serde::Serializer;
use structopt::StructOpt;

use crate::common::Run;

#[tokio::main]
async fn main() {
    let opt = options::Command::from_args();

    opt.run(&mut <dyn Serializer>::erase(
        &mut serde_json::Serializer::pretty(stdout()),
    ))
    .await
    .unwrap();

    println!();
}
