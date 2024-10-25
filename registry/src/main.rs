use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    middleware::{from_fn, Compress, Logger, NormalizePath, TrailingSlash},
    rt::System,
    web, App, HttpServer,
};
use log::info;
use std::{env::current_dir, fs::create_dir_all, path::PathBuf, sync::Mutex};

use pesde::{
    source::{pesde::PesdePackageSource, traits::PackageSource},
    AuthConfig, Project,
};

use crate::{
    auth::{get_auth_from_env, Auth, UserIdExtractor},
    search::make_search,
    storage::{get_storage_from_env, Storage},
};

mod auth;
mod endpoints;
mod error;
mod package;
mod search;
mod storage;

pub fn make_reqwest() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap()
}

pub struct AppState {
    pub source: Mutex<PesdePackageSource>,
    pub project: Project,
    pub storage: Storage,
    pub auth: Auth,

    pub search_reader: tantivy::IndexReader,
    pub search_writer: Mutex<tantivy::IndexWriter>,
}

#[macro_export]
macro_rules! benv {
    ($name:expr) => {
        std::env::var($name)
    };
    ($name:expr => $default:expr) => {
        benv!($name).unwrap_or($default.to_string())
    };
    (required $name:expr) => {
        benv!($name).expect(concat!("Environment variable `", $name, "` must be set"))
    };
    (parse $name:expr) => {
        benv!($name)
            .map(|v| v.parse().expect(concat!(
                "Environment variable `",
                $name,
                "` must be a valid value"
            )))
    };
    (parse required $name:expr) => {
        benv!(parse $name).expect(concat!("Environment variable `", $name, "` must be set"))
    };
    (parse $name:expr => $default:expr) => {
        benv!($name => $default)
            .parse()
            .expect(concat!(
                "Environment variable `",
                $name,
                "` must a valid value"
            ))
    };
}

async fn run() -> std::io::Result<()> {
    let address = benv!("ADDRESS" => "127.0.0.1");
    let port: u16 = benv!(parse "PORT" => "8080");

    let cwd = current_dir().unwrap();
    let data_dir = cwd.join("data");
    create_dir_all(&data_dir).unwrap();

    let project = Project::new(
        &cwd,
        None::<PathBuf>,
        data_dir.join("project"),
        &cwd,
        AuthConfig::new().with_git_credentials(Some(gix::sec::identity::Account {
            username: benv!(required "GITHUB_USERNAME"),
            password: benv!(required "GITHUB_PAT"),
        })),
    );
    let source = PesdePackageSource::new(benv!(required "INDEX_REPO_URL").try_into().unwrap());
    source.refresh(&project).expect("failed to refresh source");

    let (search_reader, search_writer) = make_search(&project, &source);

    let app_data = web::Data::new(AppState {
        storage: {
            let storage = get_storage_from_env();
            info!("storage: {storage}");
            storage
        },
        auth: {
            let auth =
                get_auth_from_env(source.config(&project).expect("failed to get index config"));
            info!("auth: {auth}");
            auth
        },
        source: Mutex::new(source),
        project,

        search_reader,
        search_writer: Mutex::new(search_writer),
    });

    let publish_governor_config = GovernorConfigBuilder::default()
        .key_extractor(UserIdExtractor)
        .burst_size(12)
        .seconds_per_request(60)
        .use_headers()
        .finish()
        .unwrap();

    info!("listening on {address}:{port}");

    HttpServer::new(move || {
        App::new()
            .wrap(sentry_actix::Sentry::new())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(Cors::permissive())
            .wrap(Logger::default())
            .wrap(Compress::default())
            .app_data(app_data.clone())
            .route(
                "/",
                web::get().to(|| async {
                    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"))
                }),
            )
            .service(
                web::scope("/v0")
                    .route(
                        "/search",
                        web::get()
                            .to(endpoints::search::search_packages)
                            .wrap(from_fn(auth::read_mw)),
                    )
                    .route(
                        "/packages/{name}",
                        web::get()
                            .to(endpoints::package_versions::get_package_versions)
                            .wrap(from_fn(auth::read_mw)),
                    )
                    .route(
                        "/packages/{name}/{version}/{target}",
                        web::get()
                            .to(endpoints::package_version::get_package_version)
                            .wrap(from_fn(auth::read_mw)),
                    )
                    .route(
                        "/packages",
                        web::post()
                            .to(endpoints::publish_version::publish_package)
                            .wrap(Governor::new(&publish_governor_config))
                            .wrap(from_fn(auth::write_mw)),
                    ),
            )
    })
    .bind((address, port))?
    .run()
    .await
}

// can't use #[actix_web::main] because of Sentry:
// "Note: Macros like #[tokio::main] and #[actix_web::main] are not supported. The Sentry client must be initialized before the async runtime is started so that all threads are correctly connected to the Hub."
// https://docs.sentry.io/platforms/rust/guides/actix-web/
fn main() -> std::io::Result<()> {
    let _ = dotenvy::dotenv();

    let mut log_builder = pretty_env_logger::formatted_builder();
    log_builder.parse_env(pretty_env_logger::env_logger::Env::default().default_filter_or("info"));

    let logger = sentry_log::SentryLogger::with_dest(log_builder.build());
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let guard = sentry::init(sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
    });

    if guard.is_enabled() {
        std::env::set_var("RUST_BACKTRACE", "full");
    }

    System::new().block_on(run())
}
