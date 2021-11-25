use structopt::StructOpt;

use crate::{run_impl_enum, run_impl_struct};

#[derive(StructOpt)]
pub struct Ebay {
    #[structopt(subcommand)]
    query_type: QueryType,
}

run_impl_struct!(Ebay, query_type);

#[derive(StructOpt)]
enum QueryType {
    Product(product::SubCommand),
}

run_impl_enum!(QueryType, self, ser, {
    match self {
        Self::Product(p) => p.run(ser).await?,
    }
});

mod product {
    use crate::run_impl_enum;
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub(super) enum SubCommand {
        Id { id: u64 },
    }

    run_impl_enum!(SubCommand, self, ser, {
        match self {
            Self::Id { id } => {
                erased_serde::serialize(
                    &datacollect::modules::ebay::Product::by_id(&mut Default::default(), *id)
                        .await?,
                    ser,
                )?;
            }
        }
    });
}
