#![deny(elided_lifetimes_in_paths)]
mod data_struct;
mod db;
mod handler;
mod notifier;

use std::{fs::create_dir_all, sync::Arc};

use actix_cors::Cors;
use actix_files as fs;
use actix_web_httpauth::extractors::bearer;
use anyhow::Result;
use clap::Parser;
use db::StorageType;
use handler::AppState;
use log::{error, info};
use serde::Serialize;
use simple_on_shutdown::on_shutdown;
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};

#[derive(Parser)]
#[command(
    author("https://github.com/mmta"),
    version,
    about = "Ywinby server",
    long_about = "Ywinby server\n\nA system that keeps one share of the 2-of-3 Shamir's secret \
                  sharing system,\nand release it to the recipient if the owner fails respond \
                  after a certain time."
)]
struct Args {
    /// Scheduled task period
    #[arg(short('t'), long, env, value_name = "seconds", default_value_t = 3600)]
    scheduled_task_period: u64,
    #[arg(value_enum)]
    /// Storage type to use
    #[arg(short('s'), long, env, default_value = "json")]
    storage: StorageType,
    /// GCP Project ID if using Firestore
    #[arg(short('j'), long, env, default_value = "", required_if_eq("storage", "firestore"))]
    project_id: String,
    /// Base64 VAPID private key for web push notification.
    #[arg(
        short('k'),
        long,
        env,
        value_name = "strings",
        required_unless_present("generate"),
        default_value = ""
    )]
    push_privkey: String,
    /// Public key of the above to be used by web clients, will be written to
    /// runtime-config.json.
    #[arg(
        short('p'),
        long,
        env,
        value_name = "strings",
        required_unless_present("generate"),
        default_value = ""
    )]
    push_pubkey: String,
    /// The URL that web clients use to contact this server, will be written to
    /// runtime-config.json.
    #[arg(short('u'), long, env, value_name = "url", default_value = "http://localhost:8080")]
    base_api_path: String,
    /// Google oAuth2 Client ID of the app that users will be signing in to.
    #[arg(
        short('c'),
        long,
        env,
        value_name = "client_id",
        default_value = "806452214643-l366imhlc0c64coebiik6t3otfjatis3.apps.googleusercontent.com"
    )]
    client_id: String,
    /// Block new user registration
    #[arg(short('b'), env)]
    block_registration: bool,
    /// Activate serverless mode, and authenticate request for scheduled task
    /// using this token
    #[arg(short('e'), long("serverless_token"), env, default_value = "")]
    serverless_token: String,
    /// Generate new VAPID private and public keys
    #[arg(short('g'), long("generate"))]
    generate: bool,
    /// Increase logging verbosity
    #[arg(short('v'), long, action = clap::ArgAction::Count)]
    verbosity: u8,
}

fn update_client_config(api_path: String, pubkey: String) -> Result<()> {
    let cfg = WebClientRuntimeConfig { api_url: api_path, push_pubkey_base64: pubkey };
    let dir = std::env::current_exe()?.parent().unwrap().join("static");
    create_dir_all(&dir)?;
    let file = dir.join("runtime-config.json");
    let cfg_str = serde_json::to_string_pretty(&cfg)?;
    std::fs::write(file, cfg_str)?;
    Ok(())
}

fn generate_keys() -> Result<()> {
    let k = vapid::Key::generate().map_err(|err| {
        error!("exiting, cannot generate key: {:?}", err);
        err
    })?;
    println!(
        "These can be used for push_privkey (-k) and push_pubkey (-p) parameters:\n- privateKey: \
         {}\n- publicKey: {}
",
        k.to_private_raw(),
        k.to_public_raw()
    );
    Ok(())
}

#[derive(Serialize)]
struct WebClientRuntimeConfig {
    api_url: String,
    push_pubkey_base64: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.generate {
        return generate_keys();
    }

    let log_severity = match args.verbosity {
        0 => Severity::Info,
        1 => Severity::Debug,
        _ => Severity::Trace,
    };
    let logger = TerminalLoggerBuilder::new()
        .level(log_severity)
        .destination(Destination::Stderr)
        .build()
        .map_err(|err| {
            error!("exiting, cannot configure logger: {:?}", err);
            err
        })?;
    let _guard = sloggers::set_stdlog_logger(logger).map_err(|err| {
        error!("exiting, cannot set standard logger: {:?}", err);
        err
    })?;

    update_client_config(args.base_api_path, args.push_pubkey).map_err(|err| {
        error!("exiting, cannot update client config: {:?}", err);
        err
    })?;

    let storage_id =
        if args.storage == StorageType::Firestore { args.project_id } else { "db".to_string() };

    on_shutdown!(|| {
        _guard.cancel_reset();
        info!("server is shutting down")
    });

    if !args.serverless_token.is_empty() {
        info!("starting in serverless mode, not activating scheduler");
    } else {
        tokio::spawn(async move {
            notifier::start_scheduler(
                args.storage,
                &storage_id,
                args.scheduled_task_period,
                args.push_privkey,
            )
            .await
        });
    }

    use actix_web::{App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allowed_methods(["GET", "POST", "DELETE", "CONNECT", "OPTIONS"])
                    .disable_vary_header(),
            )
            .data_factory(|| async {
                let state = data_factory_creator().await.unwrap();
                Ok::<_, ()>(state)
            })
            .app_data(bearer::Config::default().realm("Registered-users only").scope("Ywinby"))
            .service(handler::message_list)
            .service(handler::message_create)
            .service(handler::message_delete)
            .service(handler::user_pong)
            .service(handler::subscribe_user)
            .service(handler::unsubscribe_user)
            .service(handler::test_notification)
            .service(handler::serverless_scheduled_task)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .workers(1)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;
    Ok(())
}

async fn data_factory_creator() -> Result<AppState> {
    let args = Args::parse();
    let s_id =
        if args.storage == StorageType::Firestore { args.project_id } else { "db".to_string() };
    let web_pusher = Arc::new(notifier::WebPusher::new(args.push_privkey)?);
    let sdb = db::DBBuilder::new(args.storage, &s_id).await?;

    Ok(handler::AppState {
        db: sdb,
        web_push: web_pusher.to_owned(),
        block_registration: args.block_registration,
        scheduled_task_period: args.scheduled_task_period,
        oauth_client_id: args.client_id.to_owned(),
        serverless_token: args.serverless_token.to_owned(),
        scheduled_task_running: std::sync::atomic::AtomicBool::new(false),
    })
}
