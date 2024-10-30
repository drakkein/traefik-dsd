use anyhow::Result;
use redis::{Client, Connection, RedisResult};

pub struct RedisClient {
    connection: Connection,
}

impl RedisClient {
    pub fn new(url: &str) -> Result<Self> {
        let client = Client::open(url)?;
        let connection = client.get_connection()?;
        Ok(Self { connection })
    }

    pub fn set_key(&mut self, key: &str, value: &str, expire_seconds: u64) -> RedisResult<()> {
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query(&mut self.connection)?;
        redis::cmd("EXPIRE")
            .arg(key)
            .arg(expire_seconds)
            .query(&mut self.connection)?;
        Ok(())
    }
}