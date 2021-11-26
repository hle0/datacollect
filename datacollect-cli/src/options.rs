use crate::{
    modules::{ebay::Ebay, passmark::Passmark, rdap::Rdap},
    run_impl_enum,
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "datacollect-cli")]
pub enum Command {
    Passmark(Passmark),
    Ebay(Ebay),
    Rdap(Rdap),
}

run_impl_enum!(Command, self, ser, {
    match self {
        Self::Passmark(p) => p.run(ser).await?,
        Self::Ebay(e) => e.run(ser).await?,
        Self::Rdap(r) => r.run(ser).await?,
    }
});
