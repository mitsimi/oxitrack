use std::env;

pub struct Config {
    database_url: String,
    port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let database_url = env::var("DATABASE_URL").unwrap_or(String::from("sqlite:oxitrack.db"));

        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self { database_url, port }
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
