use structopt::StructOpt;

use crate::{run_impl_enum, run_impl_struct};

#[derive(StructOpt)]
pub struct Rdap {
    #[structopt(subcommand)]
    query_type: QueryType,
}

run_impl_struct!(Rdap, query_type);

#[derive(StructOpt)]
enum QueryType {
    Domain(domain::SubCommand),
}

run_impl_enum!(QueryType, self, ser, {
    match self {
        Self::Domain(d) => d.run(ser).await?,
    }
});

mod domain {
    use crate::run_impl_enum;
    use datacollect::chrono::Utc;
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub(super) enum SubCommand {
        Json { name: String },
        IsRegistered { name: String },
        IsLocked { name: String },
        CanPurchase { name: String },
    }

    run_impl_enum!(SubCommand, self, ser, {
        match self {
            Self::Json { name } => {
                erased_serde::serialize(
                    &datacollect::modules::rdap::DomainRecord::get(&mut Default::default(), name)
                        .await?,
                    ser,
                )?;
            }
            Self::IsRegistered { name } => {
                erased_serde::serialize(
                    &datacollect::modules::rdap::DomainRecord::get(&mut Default::default(), name)
                        .await?
                        .map(|record| record.is_registered_at(&Utc::now()))
                        .unwrap_or(false),
                    ser,
                )?;
            }
            Self::IsLocked { name } => {
                erased_serde::serialize(
                    &datacollect::modules::rdap::DomainRecord::get(&mut Default::default(), name)
                        .await?
                        .map(|record| record.is_locked_at(&Utc::now()))
                        .unwrap_or(false),
                    ser,
                )?;
            }
            Self::CanPurchase { name } => {
                erased_serde::serialize(
                    &datacollect::modules::rdap::DomainRecord::get(&mut Default::default(), name)
                        .await?
                        .map(|record| record.is_buyable_at(&Utc::now()))
                        .unwrap_or(true),
                    ser,
                )?;
            }
        }
    });
}
