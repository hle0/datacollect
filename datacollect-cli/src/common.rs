use async_trait::async_trait;
use erased_serde::Serializer;

#[async_trait]
pub trait Run {
    async fn run(&self, serializer: &mut (dyn Serializer + Send)) -> anyhow::Result<()>;
}

#[macro_export]
macro_rules! run_impl_enum {
    ($i:ident, $self:ident, $ser:ident, $b:block) => {
        #[async_trait::async_trait]
        impl $crate::common::Run for $i {
            async fn run(&$self, $ser: &mut (dyn erased_serde::Serializer + Send)) -> anyhow::Result<()> {
                $b

                Ok(())
            }
        }
    }
}

#[macro_export]
macro_rules! run_impl_struct {
    ($i:ident, $b:ident) => {
        #[async_trait::async_trait]
        impl $crate::common::Run for $i {
            async fn run(
                &self,
                serializer: &mut (dyn erased_serde::Serializer + Send),
            ) -> anyhow::Result<()> {
                self.$b.run(serializer).await
            }
        }
    };
}
