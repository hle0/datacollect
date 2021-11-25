use crate::{run_impl_enum, run_impl_struct};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Passmark {
    #[structopt(subcommand)]
    data_type: DataType,
}

run_impl_struct!(Passmark, data_type);

#[derive(StructOpt)]
enum DataType {
    Cpu(cpu::SubCommand),
}

run_impl_enum!(DataType, self, ser, {
    match self {
        Self::Cpu(cpu) => cpu.run(ser).await?,
    }
});

mod cpu {
    use crate::run_impl_enum;
    use structopt::StructOpt;

    #[derive(StructOpt)]
    pub(super) enum SubCommand {
        MegaList,
    }

    run_impl_enum!(SubCommand, self, ser, {
        match self {
            Self::MegaList => {
                erased_serde::serialize(
                    &datacollect::modules::passmark::CPUMegaList::get(&mut Default::default())
                        .await?,
                    ser,
                )?;
            }
        }
    });
}
