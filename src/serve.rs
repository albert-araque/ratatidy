//! `ratatidy serve`: a read-only web catalog on the Host.
//! It listens on a loopback address and never deletes or transfers.

use crate::catalog::{self, Listing, Side};
use crate::config::{Config, QBittorrentConfig};
use crate::grouping::{self, Group};
use crate::qbittorrent::{self, QbitClient, RealQbitClient, TorrentInfo};
use crate::scanner::{FileNode, ScanEvent, Scanner};
use anyhow::{Result, anyhow};
use askama::Template;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub async fn run(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        eprintln!(
            "\
Usage: ratatidy serve [--bind 127.0.0.1:8787]

Read-only web catalog of this Host's library. Loopback addresses only.
RATATIDY_BIND is used when --bind is omitted."
        );
        return Ok(());
    }

    let addr = bind_addr(args)?;
    let config = Config::load_from_file().unwrap_or_else(Config::from_env);
    let download_dir = config
        .download_dir
        .clone()
        .ok_or_else(|| anyhow!("download_dir is not configured on this host"))?;
    let torrents = fetch_torrents(&config.qbittorrent).await;
    let torrent_list = match &torrents {
        Fetch::List(list) => list.clone(),
        Fetch::Failed => Vec::new(),
    };

    let snapshot = Arc::new(RwLock::new(Snapshot {
        download_dir,
        media_dirs: config.media_dirs.clone(),
        nodes: Vec::new(),
        media_groups: vec![Vec::new(); config.media_dirs.len()],
        download_groups: Vec::new(),
        torrents: torrent_list,
        processed: 0,
        phase: Phase::Scanning,
    }));
    let scanning = Arc::new(AtomicBool::new(false));
    let state = Arc::new(ServeState {
        snapshot: Arc::clone(&snapshot),
        scanning: Arc::clone(&scanning),
        qbit: config.qbittorrent.clone(),
    });
    start_scan(&state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Ratatidy catalog on http://{addr}");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub struct ServeState {
    snapshot: Arc<RwLock<Snapshot>>,
    scanning: Arc<AtomicBool>,
    qbit: QBittorrentConfig,
}

pub struct Snapshot {
    pub download_dir: PathBuf,
    pub media_dirs: Vec<PathBuf>,
    pub nodes: Vec<FileNode>,
    pub media_groups: Vec<Vec<Group>>,
    pub download_groups: Vec<Group>,
    pub torrents: Vec<TorrentInfo>,
    pub processed: usize,
    pub phase: Phase,
}

#[derive(Clone)]
pub enum Phase {
    Scanning,
    Ready,
    Failed(String),
}

enum Fetch {
    List(Vec<TorrentInfo>),
    Failed,
}

async fn fetch_torrents(config: &QBittorrentConfig) -> Fetch {
    match RealQbitClient::new(
        &config.url,
        config.username.clone(),
        config.password.clone(),
    )
    .await
    {
        Ok(client) => match client.get_torrents().await {
            Ok(list) => Fetch::List(list),
            Err(_) => Fetch::Failed,
        },
        Err(_) => Fetch::Failed,
    }
}

fn start_scan(state: &ServeState) {
    if state
        .scanning
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    {
        let mut snap = state
            .snapshot
            .write()
            .unwrap_or_else(|err| err.into_inner());
        snap.phase = Phase::Scanning;
        snap.processed = 0;
    }
    let snapshot = Arc::clone(&state.snapshot);
    let scanning = Arc::clone(&state.scanning);
    std::thread::spawn(move || {
        let (download_dir, media_dirs) = {
            let snap = snapshot.read().unwrap_or_else(|err| err.into_inner());
            (snap.download_dir.clone(), snap.media_dirs.clone())
        };
        let (tx, rx) = std::sync::mpsc::channel();
        Scanner::new(download_dir.clone(), media_dirs.clone()).scan_async(tx);
        let mut processed = 0usize;
        let mut finished = false;
        for event in rx {
            match event {
                ScanEvent::FileScanned => {
                    processed += 1;
                    if processed % 100 == 0 {
                        let mut snap = snapshot.write().unwrap_or_else(|err| err.into_inner());
                        snap.processed = processed;
                    }
                }
                ScanEvent::Finished(mut nodes) => {
                    let torrents = {
                        let snap = snapshot.read().unwrap_or_else(|err| err.into_inner());
                        snap.torrents.clone()
                    };
                    qbittorrent::match_torrents(&mut nodes, &torrents);
                    let media_groups: Vec<Vec<Group>> = media_dirs
                        .iter()
                        .map(|dir| grouping::group_by_media(&nodes, std::slice::from_ref(dir)))
                        .collect();
                    let download_groups = grouping::group_by_downloads(&nodes, &download_dir);
                    let mut snap = snapshot.write().unwrap_or_else(|err| err.into_inner());
                    snap.nodes = nodes;
                    snap.media_groups = media_groups;
                    snap.download_groups = download_groups;
                    snap.processed = processed;
                    snap.phase = Phase::Ready;
                    finished = true;
                    break;
                }
                ScanEvent::Error(error) => {
                    let mut snap = snapshot.write().unwrap_or_else(|err| err.into_inner());
                    snap.processed = processed;
                    snap.phase = Phase::Failed(error);
                    finished = true;
                    break;
                }
            }
        }
        if !finished {
            let mut snap = snapshot.write().unwrap_or_else(|err| err.into_inner());
            snap.phase = Phase::Failed("el escaneo terminó sin resultado".into());
        }
        scanning.store(false, Ordering::SeqCst);
    });
}

fn router(state: Arc<ServeState>) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/library/{index}", get(library))
        .route("/downloads", get(downloads))
        .route("/group", get(group))
        .route("/rescan", post(rescan))
        .route("/assets/app.css", get(css))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    nav: Vec<NavLink>,
    scan_message: String,
    scanning: bool,
    summary: catalog::Summary,
    orphans: Vec<catalog::Card>,
    more_orphans: usize,
}

#[derive(Template)]
#[template(path = "catalog.html")]
struct CatalogPage {
    nav: Vec<NavLink>,
    scan_message: String,
    scanning: bool,
    heading: String,
    folder: String,
    action: String,
    q: String,
    filter_value: String,
    sort_value: String,
    order_value: String,
    filters: Vec<catalog::Chip>,
    sorts: Vec<catalog::Chip>,
    cards: Vec<catalog::Card>,
}

#[derive(Template)]
#[template(path = "group.html")]
struct GroupPage {
    nav: Vec<NavLink>,
    scan_message: String,
    scanning: bool,
    detail: catalog::Detail,
}

#[derive(Template)]
#[template(path = "missing.html")]
struct MissingPage {
    nav: Vec<NavLink>,
    scan_message: String,
    scanning: bool,
    back_href: String,
    back_label: String,
}

struct NavLink {
    href: String,
    label: String,
    active: bool,
}

#[derive(Clone, Copy)]
enum Active {
    Home,
    Library(usize),
    Downloads,
}

#[derive(Deserialize)]
struct CatalogParams {
    #[serde(default)]
    q: String,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    sort: String,
    #[serde(default)]
    order: String,
}

#[derive(Deserialize)]
struct GroupParams {
    side: String,
    path: String,
}

async fn home(State(state): State<Arc<ServeState>>) -> Response {
    let snap = state.snapshot.read().unwrap_or_else(|err| err.into_inner());
    let (scan_message, scanning) = scan_status(&snap);
    let (orphans, more_orphans) =
        catalog::top_orphans(&snap.media_groups, &snap.download_groups, 8);
    page(&HomePage {
        nav: nav(&snap, Active::Home),
        scan_message,
        scanning,
        summary: catalog::summary(&snap.nodes),
        orphans,
        more_orphans,
    })
}

async fn library(
    State(state): State<Arc<ServeState>>,
    Path(index): Path<usize>,
    Query(params): Query<CatalogParams>,
) -> Response {
    let snap = state.snapshot.read().unwrap_or_else(|err| err.into_inner());
    let Some(dir) = snap.media_dirs.get(index) else {
        return missing(&snap, Active::Home, "/", "Inicio");
    };
    let groups = snap
        .media_groups
        .get(index)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    catalog_page(
        &snap,
        Active::Library(index),
        folder_label(dir, &snap.media_dirs),
        dir.display().to_string(),
        format!("/library/{index}"),
        groups,
        Side::Media,
        &params,
    )
}

async fn downloads(
    State(state): State<Arc<ServeState>>,
    Query(params): Query<CatalogParams>,
) -> Response {
    let snap = state.snapshot.read().unwrap_or_else(|err| err.into_inner());
    catalog_page(
        &snap,
        Active::Downloads,
        "Descargas".to_string(),
        snap.download_dir.display().to_string(),
        "/downloads".to_string(),
        &snap.download_groups,
        Side::Downloads,
        &params,
    )
}

fn catalog_page(
    snap: &Snapshot,
    active: Active,
    heading: String,
    folder: String,
    action: String,
    groups: &[Group],
    side: Side,
    params: &CatalogParams,
) -> Response {
    let listing = Listing::parse(&params.q, &params.filter, &params.sort, &params.order);
    let (scan_message, scanning) = scan_status(snap);
    page(&CatalogPage {
        nav: nav(snap, active),
        scan_message,
        scanning,
        heading,
        folder,
        filters: catalog::filter_chips(&action, &listing),
        sorts: catalog::sort_chips(&action, &listing),
        cards: catalog::cards(groups, side, &listing),
        q: listing.q,
        filter_value: listing.filter.as_str().to_string(),
        sort_value: listing.sort.as_str().to_string(),
        order_value: listing.order.as_str().to_string(),
        action,
    })
}

async fn group(
    State(state): State<Arc<ServeState>>,
    Query(params): Query<GroupParams>,
) -> Response {
    let snap = state.snapshot.read().unwrap_or_else(|err| err.into_inner());
    let Some(side) = Side::parse(&params.side) else {
        return missing(&snap, Active::Home, "/", "Inicio");
    };
    let found = match side {
        Side::Media => {
            catalog::find_media(&snap.media_groups, &params.path).map(|(index, group)| {
                let label = snap
                    .media_dirs
                    .get(index)
                    .map(|dir| folder_label(dir, &snap.media_dirs))
                    .unwrap_or_else(|| "Media".into());
                (
                    group,
                    Active::Library(index),
                    format!("/library/{index}"),
                    label,
                )
            })
        }
        Side::Downloads => {
            catalog::find_download(&snap.download_groups, &params.path).map(|group| {
                (
                    group,
                    Active::Downloads,
                    "/downloads".to_string(),
                    "Descargas".to_string(),
                )
            })
        }
    };
    let Some((group, active, back_href, back_label)) = found else {
        let active = match side {
            Side::Media => Active::Home,
            Side::Downloads => Active::Downloads,
        };
        let (href, label) = match side {
            Side::Media => ("/", "Inicio"),
            Side::Downloads => ("/downloads", "Descargas"),
        };
        return missing(&snap, active, href, label);
    };
    let (scan_message, scanning) = scan_status(&snap);
    page(&GroupPage {
        nav: nav(&snap, active),
        scan_message,
        scanning,
        detail: catalog::detail(
            group,
            &snap.download_dir,
            &snap.torrents,
            &back_href,
            &back_label,
        ),
    })
}

async fn rescan(State(state): State<Arc<ServeState>>, headers: HeaderMap) -> Response {
    if !state.scanning.load(Ordering::SeqCst) {
        if let Fetch::List(list) = fetch_torrents(&state.qbit).await {
            let mut snap = state
                .snapshot
                .write()
                .unwrap_or_else(|err| err.into_inner());
            snap.torrents = list;
        }
        start_scan(&state);
    }
    let target = redirect_target(
        headers
            .get(header::REFERER)
            .and_then(|value| value.to_str().ok()),
    );
    Redirect::to(&target).into_response()
}

async fn css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../assets/serve.css"),
    )
}

fn missing(snap: &Snapshot, active: Active, back_href: &str, back_label: &str) -> Response {
    let (scan_message, scanning) = scan_status(snap);
    let page = MissingPage {
        nav: nav(snap, active),
        scan_message,
        scanning,
        back_href: back_href.to_string(),
        back_label: back_label.to_string(),
    };
    match page.render() {
        Ok(body) => (StatusCode::NOT_FOUND, Html(body)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn page<T: Template>(tmpl: &T) -> Response {
    match tmpl.render() {
        Ok(body) => Html(body).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn scan_status(snap: &Snapshot) -> (String, bool) {
    match &snap.phase {
        Phase::Scanning => (
            format!(
                "Escaneando la biblioteca… {} ficheros vistos. La página se actualizará sola.",
                snap.processed
            ),
            true,
        ),
        Phase::Ready => (String::new(), false),
        Phase::Failed(error) if snap.nodes.is_empty() => {
            (format!("No se pudo escanear: {error}"), false)
        }
        Phase::Failed(error) => (
            format!("El último escaneo falló ({error}). Se muestra la foto anterior."),
            false,
        ),
    }
}

fn nav(snap: &Snapshot, active: Active) -> Vec<NavLink> {
    let mut links = vec![NavLink {
        href: "/".into(),
        label: "Inicio".into(),
        active: matches!(active, Active::Home),
    }];
    for (index, dir) in snap.media_dirs.iter().enumerate() {
        links.push(NavLink {
            href: format!("/library/{index}"),
            label: folder_label(dir, &snap.media_dirs),
            active: matches!(active, Active::Library(n) if n == index),
        });
    }
    links.push(NavLink {
        href: "/downloads".into(),
        label: "Descargas".into(),
        active: matches!(active, Active::Downloads),
    });
    links
}

fn folder_label(dir: &std::path::Path, all: &[PathBuf]) -> String {
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    let duplicated = all
        .iter()
        .filter(|other| other.file_name() == dir.file_name())
        .count()
        > 1;
    if duplicated {
        dir.display().to_string()
    } else {
        name
    }
}

fn bind_addr(args: &[String]) -> Result<SocketAddr> {
    let mut bind = std::env::var("RATATIDY_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--bind" {
            bind = args
                .get(index + 1)
                .cloned()
                .ok_or_else(|| anyhow!("--bind needs an address, for example 127.0.0.1:8787"))?;
            index += 2;
        } else {
            return Err(anyhow!(
                "unknown argument '{}'. Usage: ratatidy serve [--bind 127.0.0.1:8787]",
                args[index]
            ));
        }
    }
    let addr: SocketAddr = bind
        .parse()
        .map_err(|_| anyhow!("could not parse bind address '{bind}'"))?;
    if !addr.ip().is_loopback() {
        return Err(anyhow!(
            "serve only listens on a loopback address (got {addr})"
        ));
    }
    Ok(addr)
}

fn redirect_target(referer: Option<&str>) -> String {
    let Some(value) = referer else {
        return "/".to_string();
    };
    let path = if let Some(scheme) = value.find("://") {
        let after_host = &value[scheme + 3..];
        match after_host.find('/') {
            Some(slash) => &after_host[slash..],
            None => return "/".to_string(),
        }
    } else if let Some(stripped) = value.strip_prefix('/') {
        if stripped.starts_with('/') {
            return "/".to_string();
        }
        value
    } else {
        return "/".to_string();
    };
    let path = path.split('#').next().unwrap_or("/");
    if path.starts_with('/') && !path.starts_with("//") {
        path.to_string()
    } else {
        "/".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grouping::GroupKind;
    use crate::scanner::FileKey;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use std::time::SystemTime;
    use tower::ServiceExt;

    fn node(inode: u64, size: u64, paths: &[&str], downloads: bool, media: bool) -> FileNode {
        FileNode {
            key: FileKey { dev: 1, inode },
            size,
            nlink: 1,
            paths: paths.iter().map(PathBuf::from).collect(),
            has_downloads: downloads,
            has_media: media,
            torrent_hash: None,
            is_seeding: false,
            modified: Some(SystemTime::UNIX_EPOCH),
        }
    }

    fn sample_state() -> Arc<ServeState> {
        let inception = "/media/movies/Inception";
        let loose = "/downloads/Loose";
        Arc::new(ServeState {
            snapshot: Arc::new(RwLock::new(Snapshot {
                download_dir: PathBuf::from("/downloads"),
                media_dirs: vec![PathBuf::from("/media/movies")],
                nodes: vec![
                    node(1, 1000, &["/media/movies/Inception/a.mkv"], true, true),
                    node(2, 400, &["/downloads/Loose/b.mkv"], true, false),
                ],
                media_groups: vec![vec![Group {
                    kind: GroupKind::Movie,
                    title: "Inception".into(),
                    media_container: Some(PathBuf::from(inception)),
                    downloads_container: None,
                    nodes: vec![node(
                        1,
                        1000,
                        &[
                            "/media/movies/Inception/a.mkv",
                            "/downloads/Inception/a.mkv",
                        ],
                        true,
                        true,
                    )],
                }]],
                download_groups: vec![Group {
                    kind: GroupKind::DownloadTorrent,
                    title: "Loose".into(),
                    media_container: None,
                    downloads_container: Some(PathBuf::from(loose)),
                    nodes: vec![node(2, 400, &["/downloads/Loose/b.mkv"], true, false)],
                }],
                torrents: vec![],
                processed: 2,
                phase: Phase::Ready,
            })),
            scanning: Arc::new(AtomicBool::new(false)),
            qbit: QBittorrentConfig {
                url: "http://127.0.0.1:9".into(),
                username: None,
                password: None,
            },
        })
    }

    async fn body_of(response: Response) -> (StatusCode, String) {
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[test]
    fn loopback_bind_and_redirect_stay_on_this_host() {
        let addr = bind_addr(&["--bind".into(), "127.0.0.1:9".into()]).unwrap();
        assert!(addr.ip().is_loopback());
        assert!(bind_addr(&["--bind".into(), "0.0.0.0:8787".into()]).is_err());
        assert_eq!(
            redirect_target(Some("http://127.0.0.1:8787/library/0?filter=orphans")),
            "/library/0?filter=orphans"
        );
        assert_eq!(
            redirect_target(Some("https://evil.example/library/0")),
            "/library/0"
        );
        assert_eq!(redirect_target(Some("//evil.example")), "/");
        assert_eq!(redirect_target(None), "/");
    }

    #[tokio::test]
    async fn routes_serve_the_catalog_and_miss_unknown_groups() {
        let app = router(sample_state());
        let (status, home) = body_of(
            app.clone()
                .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(home.contains("Ficheros"));
        assert!(home.contains("Loose"));

        let (status, library) = body_of(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/library/0")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(library.contains("Inception"));

        let href = catalog::group_href(
            Side::Media,
            PathBuf::from("/media/movies/Inception").as_path(),
        );
        let (status, group) = body_of(
            app.clone()
                .oneshot(Request::builder().uri(&href).body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(group.contains("Inception"));
        assert!(group.contains("Rutas en media"));

        let (status, _) = body_of(
            app.oneshot(
                Request::builder()
                    .uri("/group?side=media&path=%2Fmissing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
