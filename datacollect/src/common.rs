use async_trait::async_trait;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Depth {
    Less,
    Default,
    More,
}

#[async_trait]
pub trait DataProducer<T> {
    async fn produce(&mut self, depth: Depth) -> anyhow::Result<T>;
}

#[async_trait]
pub trait DataConsumer<T> {
    async fn consume(&mut self, sinks: T) -> anyhow::Result<()>
    where
        T: 'async_trait;
}

#[async_trait]
pub trait DataProcessor<Sources = (), Sinks = ()> {
    async fn process(&mut self, sinks: Sinks, depth: Depth) -> anyhow::Result<Sources>;
}

pub struct DebugSink {}

#[async_trait]
impl<T> DataConsumer<T> for DebugSink
where
    T: std::fmt::Debug + Send + Sync,
{
    async fn consume(&mut self, sinks: T) -> anyhow::Result<()>
    where
        T: 'async_trait,
    {
        println!("{:?}", sinks);
        Ok(())
    }
}

/*
 * Convert something like "$312.03" to 31203
 * "$312.03" -> 31203
 * "312.03"  -> 31203
 * "312"     -> 31200
 * "312.009" -> 31200 (truncated)
 */
pub(crate) fn parse_dollars<T: AsRef<str>>(s: T) -> Option<u32> {
    let s = s.as_ref().to_string();
    if s.is_empty() || s.chars().filter(|c| *c == '.').count() > 1 {
        None
    } else {
        let mut i = s.split('.');
        let first = i
            .next()?
            .chars()
            .filter(|c| c.is_numeric())
            .collect::<String>();
        let last = i
            .next()
            .filter(|x| !x.is_empty())
            .unwrap_or("0")
            .chars()
            .take(2)
            .collect::<String>();
        Some(first.parse::<u32>().ok()? * 100 + last.parse::<u32>().ok()?)
    }
}

#[cfg(test)]
#[test]
fn test_parse_dollars() {
    assert_eq!(parse_dollars("$312.04").unwrap(), 31204);
    assert_eq!(parse_dollars("8.8.4.4"), None);
    assert_eq!(parse_dollars("42").unwrap(), 4200);
    assert_eq!(parse_dollars("$42.567").unwrap(), 4256);
}
