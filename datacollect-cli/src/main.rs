use anyhow::anyhow;
use clap::arg_enum;
use datacollect::{
    common::{DataProcessor, DataProducer},
    modules::{ebay::EbayItemSource, passmark::PassmarkCPUDataSource},
};
use erased_serde::Serialize;
use structopt::StructOpt;

arg_enum! {
    enum Processors {
        Passmark,
        Ebay
    }
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(possible_values = &Processors::variants(), case_insensitive = true)]
    f: Processors,
    input: Option<String>,
}

impl Processors {
    pub async fn execute(&self, opts: &Opt) -> anyhow::Result<Box<dyn Serialize>> {
        match self {
            Processors::Passmark => Ok(Box::new(
                PassmarkCPUDataSource::new()?
                    .produce(datacollect::common::Depth::Default)
                    .await?,
            )),
            Processors::Ebay => Ok(Box::new(
                EbayItemSource::new()?
                    .process(
                        opts.input
                            .as_ref()
                            .ok_or_else(|| anyhow!("must provide argument 'input' (ebay item ID)"))?
                            .parse()?,
                        datacollect::common::Depth::Default,
                    )
                    .await?,
            )),
        }
    }
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    println!(
        "{}",
        serde_json::to_string_pretty(&opt.f.execute(&opt).await.unwrap()).unwrap()
    );
}
