#[macro_use]
extern crate diesel;

mod bundle;
mod consts;
mod cron;
mod database;
mod key_manager;
mod server;
mod state;
mod types;

use clap::Parser;
use cron::run_crons;
use database::queries;
use diesel::{sqlite::SqliteConnection, Connection};
use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPublic};
use key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig, KeyManager};
use server::{run_server, RuntimeContext};
use state::{generate_state, SharedValidatorState, ValidatorStateTrait};
use std::{fs, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug, Parser)]
struct AppConfig {
    /// Do not start cron jobs
    #[clap(long)]
    no_cron: bool,

    /// Do not start app in server mode
    #[clap(long)]
    no_server: bool,

    /// Database connection URL
    #[clap(long, env, default_value = "postgres://bundlr:bundlr@127.0.0.1/bundlr")]
    database_url: String,

    /// Redis connection URL
    #[clap(long, env, default_value = "redis://127.0.0.1")]
    redis_connection_url: String,

    /// Listen address for the server
    #[clap(short, long, env, default_value = "127.0.0.1:10000")]
    listen: SocketAddr,

    /// Bundler public key as string
    #[clap(
        long,
        env = "BUNDLER_PUBLIC",
        conflicts_with = "bundler-key",
        required_unless_present = "bundler-key"
    )]
    bundler_public: Option<String>,

    /// Path to JWK file holding bundler public key
    #[clap(
        long,
        env = "BUNDLER_KEY",
        conflicts_with = "bundler-public",
        required_unless_present = "bundler-public"
    )]
    bundler_key: Option<String>,

    /// Path to JWK file holding validator private key
    #[clap(long, env = "VALIDATOR_KEY")]
    validator_key: String,
}

#[derive(Clone)]
struct AppContext {
    key_manager: Arc<InMemoryKeyManager>,
    database_url: String,
    redis_connection_url: String,
    listen: SocketAddr,
    validator_state: SharedValidatorState,
}

impl InMemoryKeyManagerConfig for (JsonWebKey, JsonWebKey) {
    fn bundler_jwk(&self) -> &JsonWebKey {
        &self.0
    }

    fn validator_jwk(&self) -> &JsonWebKey {
        &self.1
    }
}

impl AppContext {
    fn new(config: &AppConfig) -> Self {
        let bundler_jwk = if let Some(key_file_path) = &config.bundler_key {
            let file = fs::read_to_string(key_file_path).unwrap();
            file.parse().unwrap()
        } else {
            let n = config.bundler_public.as_ref().unwrap();
            JsonWebKey::new(Key::RSA {
                public: RsaPublic {
                    e: PublicExponent,
                    n: n.as_bytes().into(),
                },
                private: None,
            })
        };

        let validator_jwk: JsonWebKey = {
            let file = fs::read_to_string(&config.validator_key).unwrap();
            file.parse().unwrap()
        };

        let key_manager = InMemoryKeyManager::new(&(bundler_jwk, validator_jwk));
        let state = generate_state();

        Self {
            key_manager: Arc::new(key_manager),
            database_url: config.database_url.clone(),
            redis_connection_url: config.redis_connection_url.clone(),
            listen: config.listen,
            validator_state: state,
        }
    }
}

impl queries::RequestContext for AppContext {
    // FIXME: this should use connection pool
    fn get_db_connection(&self) -> SqliteConnection {
        SqliteConnection::establish(&self.database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", self.database_url))
    }
}

impl RuntimeContext for AppContext {
    fn database_connection_url(&self) -> &str {
        &self.database_url
    }

    fn redis_connection_url(&self) -> &str {
        &self.redis_connection_url
    }

    fn bind_address(&self) -> &SocketAddr {
        &self.listen
    }
}

impl server::routes::sign::Config<Arc<InMemoryKeyManager>> for AppContext {
    fn bundler_address(&self) -> &str {
        self.key_manager.bundler_address()
    }

    fn validator_address(&self) -> &str {
        self.key_manager.validator_address()
    }

    fn current_epoch(&self) -> i64 {
        0
    }

    fn current_block(&self) -> u128 {
        0
    }

    fn key_manager(&self) -> &Arc<InMemoryKeyManager> {
        &self.key_manager
    }
}

impl ValidatorStateTrait for AppContext {
    fn get_validator_state(&self) -> &SharedValidatorState {
        &self.validator_state
    }
}

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().ok();

    let config = AppConfig::parse();
    let ctx = AppContext::new(&config);

    if !config.no_cron {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons(ctx.clone()));
    };

    if !config.no_server {
        paris::info!("Running with server");
        run_server(ctx.clone()).await.unwrap()
    };
}
