//! Read-only projections of Groups for the web catalog.
//! The TUI keeps its own selection, filters and tabs in `App`.

use crate::grouping::Group;
use crate::qbittorrent::TorrentInfo;
use crate::scanner::{FileKey, FileNode};
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Media,
    Downloads,
}

impl Side {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "media" => Some(Side::Media),
            "downloads" => Some(Side::Downloads),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Side::Media => "media",
            Side::Downloads => "downloads",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupLink {
    Linked,
    Partial,
    Orphan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Orphans,
    Hardlinked,
}

impl Filter {
    pub fn parse(value: &str) -> Self {
        match value {
            "orphans" => Filter::Orphans,
            "hardlinked" => Filter::Hardlinked,
            _ => Filter::All,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Orphans => "orphans",
            Filter::Hardlinked => "hardlinked",
        }
    }

    fn matches(self, link: GroupLink) -> bool {
        match self {
            Filter::All => true,
            // Same rule as `App::current_groups`: any file that is not on both sides.
            Filter::Orphans => link != GroupLink::Linked,
            // Every file is on both sides.
            Filter::Hardlinked => link == GroupLink::Linked,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Size,
    Date,
}

impl Sort {
    pub fn parse(value: &str) -> Self {
        match value {
            "size" => Sort::Size,
            "date" => Sort::Date,
            _ => Sort::Name,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Sort::Name => "name",
            Sort::Size => "size",
            Sort::Date => "date",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

impl Order {
    pub fn parse(value: &str) -> Self {
        match value {
            "desc" => Order::Desc,
            _ => Order::Asc,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Order::Asc => "asc",
            Order::Desc => "desc",
        }
    }

    fn flip(self) -> Self {
        match self {
            Order::Asc => Order::Desc,
            Order::Desc => Order::Asc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Listing {
    pub q: String,
    pub filter: Filter,
    pub sort: Sort,
    pub order: Order,
}

impl Default for Listing {
    fn default() -> Self {
        Self {
            q: String::new(),
            filter: Filter::All,
            sort: Sort::Name,
            order: Order::Asc,
        }
    }
}

impl Listing {
    pub fn parse(q: &str, filter: &str, sort: &str, order: &str) -> Self {
        Self {
            q: q.to_string(),
            filter: Filter::parse(filter),
            sort: Sort::parse(sort),
            order: Order::parse(order),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub size_label: String,
    pub files_label: String,
    pub link_label: String,
    pub link_class: String,
    pub seeding: bool,
    pub href: String,
    size: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct Chip {
    pub label: String,
    pub href: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub file_count: usize,
    pub total_bytes: u64,
    pub saved_bytes: u64,
    pub orphan_files: usize,
    pub seeding_files: usize,
    pub total_size: String,
    pub saved_size: String,
}

#[derive(Debug, Clone)]
pub struct PathLine {
    pub path: String,
    pub size_label: String,
    pub link_label: String,
}

#[derive(Debug, Clone)]
pub struct TorrentLine {
    pub name: String,
    pub state: String,
    pub seeding: bool,
}

#[derive(Debug, Clone)]
pub struct Detail {
    pub title: String,
    pub size_label: String,
    pub files_label: String,
    pub link_label: String,
    pub link_class: String,
    pub seeding: bool,
    pub date_label: String,
    pub media_paths: Vec<PathLine>,
    pub download_paths: Vec<PathLine>,
    pub torrents: Vec<TorrentLine>,
    pub back_href: String,
    pub back_label: String,
}

pub fn summary(nodes: &[FileNode]) -> Summary {
    let mut total_bytes = 0;
    let mut saved_bytes = 0;
    let mut orphan_files = 0;
    let mut seeding_files = 0;
    for node in nodes {
        total_bytes += node.size;
        if node.has_downloads && node.has_media {
            saved_bytes += node.size;
        } else {
            orphan_files += 1;
        }
        if node.is_seeding {
            seeding_files += 1;
        }
    }
    Summary {
        file_count: nodes.len(),
        total_bytes,
        saved_bytes,
        orphan_files,
        seeding_files,
        total_size: format_size(total_bytes),
        saved_size: format_size(saved_bytes),
    }
}

pub fn cards(groups: &[Group], side: Side, listing: &Listing) -> Vec<Card> {
    let mut cards: Vec<Card> = groups
        .iter()
        .filter_map(|group| card_for(group, side))
        .filter(|card| listing.filter.matches(link_of_class(&card.link_class)))
        .filter(|card| {
            listing.q.is_empty()
                || card
                    .title
                    .to_lowercase()
                    .contains(&listing.q.to_lowercase())
        })
        .collect();

    cards.sort_by(|a, b| {
        let cmp = match listing.sort {
            Sort::Name => a.title.cmp(&b.title),
            Sort::Size => a.size.cmp(&b.size).then_with(|| a.title.cmp(&b.title)),
            Sort::Date => a
                .modified
                .cmp(&b.modified)
                .then_with(|| a.title.cmp(&b.title)),
        };
        if listing.order == Order::Desc {
            cmp.reverse()
        } else {
            cmp
        }
    });
    cards
}

pub fn filter_chips(base: &str, listing: &Listing) -> Vec<Chip> {
    [Filter::All, Filter::Orphans, Filter::Hardlinked]
        .into_iter()
        .map(|filter| Chip {
            label: match filter {
                Filter::All => "Todos".to_string(),
                Filter::Orphans => "Sin enlace".to_string(),
                Filter::Hardlinked => "Enlazados".to_string(),
            },
            href: listing_href(
                base,
                &Listing {
                    filter,
                    ..listing.clone()
                },
            ),
            active: listing.filter == filter,
        })
        .collect()
}

pub fn sort_chips(base: &str, listing: &Listing) -> Vec<Chip> {
    [Sort::Name, Sort::Size, Sort::Date]
        .into_iter()
        .map(|sort| {
            let order = if listing.sort == sort {
                listing.order.flip()
            } else {
                Order::Asc
            };
            let arrow = if listing.sort == sort {
                match listing.order {
                    Order::Asc => " ↑",
                    Order::Desc => " ↓",
                }
            } else {
                ""
            };
            let name = match sort {
                Sort::Name => "Nombre",
                Sort::Size => "Tamaño",
                Sort::Date => "Fecha",
            };
            Chip {
                label: format!("{name}{arrow}"),
                href: listing_href(
                    base,
                    &Listing {
                        sort,
                        order,
                        ..listing.clone()
                    },
                ),
                active: listing.sort == sort,
            }
        })
        .collect()
}

pub fn listing_href(base: &str, listing: &Listing) -> String {
    let mut pairs = Vec::new();
    if !listing.q.is_empty() {
        pairs.push(format!("q={}", encode_query(&listing.q)));
    }
    if listing.filter != Filter::All {
        pairs.push(format!("filter={}", listing.filter.as_str()));
    }
    if listing.sort != Sort::Name {
        pairs.push(format!("sort={}", listing.sort.as_str()));
    }
    if listing.order != Order::Asc {
        pairs.push(format!("order={}", listing.order.as_str()));
    }
    if pairs.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", pairs.join("&"))
    }
}

/// Groups that are not fully hardlinked, largest first. Returns at most `limit` cards and how many were left out.
pub fn top_orphans(media: &[Vec<Group>], downloads: &[Group], limit: usize) -> (Vec<Card>, usize) {
    let mut cards = Vec::new();
    for groups in media {
        cards.extend(
            groups
                .iter()
                .filter_map(|group| card_for(group, Side::Media))
                .filter(|card| link_of_class(&card.link_class) != GroupLink::Linked),
        );
    }
    cards.extend(
        downloads
            .iter()
            .filter_map(|group| card_for(group, Side::Downloads))
            .filter(|card| link_of_class(&card.link_class) != GroupLink::Linked),
    );
    cards.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.title.cmp(&b.title)));
    let extra = cards.len().saturating_sub(limit);
    cards.truncate(limit);
    (cards, extra)
}

pub fn find_media<'a>(media: &'a [Vec<Group>], path: &str) -> Option<(usize, &'a Group)> {
    media.iter().enumerate().find_map(|(index, groups)| {
        groups
            .iter()
            .find(|group| container_is(&group.media_container, path))
            .map(|group| (index, group))
    })
}

pub fn find_download<'a>(groups: &'a [Group], path: &str) -> Option<&'a Group> {
    groups
        .iter()
        .find(|group| container_is(&group.downloads_container, path))
}

fn container_is(container: &Option<std::path::PathBuf>, path: &str) -> bool {
    container
        .as_ref()
        .is_some_and(|container| container.display().to_string() == path)
}

pub fn detail(
    group: &Group,
    download_dir: &Path,
    torrents: &[TorrentInfo],
    back_href: &str,
    back_label: &str,
) -> Detail {
    let nodes = unique_nodes(group);
    let link = classify(&nodes);
    let size: u64 = nodes.iter().map(|node| node.size).sum();
    let modified = nodes.iter().filter_map(|node| node.modified).max();
    let seeding = nodes.iter().any(|node| node.is_seeding);

    let mut media_paths = Vec::new();
    let mut download_paths = Vec::new();
    for node in &nodes {
        let line_size = format_size(node.size);
        let line_link = file_link_label(node);
        for path in &node.paths {
            let line = PathLine {
                path: path.display().to_string(),
                size_label: line_size.clone(),
                link_label: line_link.to_string(),
            };
            if path.starts_with(download_dir) {
                download_paths.push(line);
            } else {
                media_paths.push(line);
            }
        }
    }

    let mut seen_hashes = HashSet::new();
    let mut torrent_lines = Vec::new();
    for node in &nodes {
        let Some(hash) = node.torrent_hash.as_deref() else {
            continue;
        };
        if !seen_hashes.insert(hash.to_string()) {
            continue;
        }
        if let Some(torrent) = torrents.iter().find(|torrent| torrent.hash == hash) {
            torrent_lines.push(TorrentLine {
                name: torrent.name.clone(),
                state: torrent.state.clone(),
                seeding: node.is_seeding,
            });
        } else {
            torrent_lines.push(TorrentLine {
                name: hash.to_string(),
                state: String::new(),
                seeding: node.is_seeding,
            });
        }
    }

    Detail {
        title: group.title.clone(),
        size_label: format_size(size),
        files_label: files_label(nodes.len()),
        link_label: link_label(link).to_string(),
        link_class: link_class(link).to_string(),
        seeding,
        date_label: format_date(modified),
        media_paths,
        download_paths,
        torrents: torrent_lines,
        back_href: back_href.to_string(),
        back_label: back_label.to_string(),
    }
}

pub fn group_href(side: Side, path: &Path) -> String {
    format!(
        "/group?side={}&path={}",
        side.as_str(),
        encode_query(&path.display().to_string())
    )
}

fn card_for(group: &Group, side: Side) -> Option<Card> {
    let path = match side {
        Side::Media => group.media_container.as_deref()?,
        Side::Downloads => group.downloads_container.as_deref()?,
    };
    let nodes = unique_nodes(group);
    let link = classify(&nodes);
    let size = nodes.iter().map(|node| node.size).sum();
    Some(Card {
        title: group.title.clone(),
        size_label: format_size(size),
        files_label: files_label(nodes.len()),
        link_label: link_label(link).to_string(),
        link_class: link_class(link).to_string(),
        seeding: nodes.iter().any(|node| node.is_seeding),
        href: group_href(side, path),
        size,
        modified: nodes.iter().filter_map(|node| node.modified).max(),
    })
}

fn unique_nodes(group: &Group) -> Vec<&FileNode> {
    let mut seen = HashSet::new();
    group
        .nodes
        .iter()
        .filter(|node| seen.insert(node.key))
        .collect()
}

fn classify(nodes: &[&FileNode]) -> GroupLink {
    let linked = nodes
        .iter()
        .filter(|node| node.has_downloads && node.has_media)
        .count();
    if nodes.is_empty() || linked == 0 {
        GroupLink::Orphan
    } else if linked == nodes.len() {
        GroupLink::Linked
    } else {
        GroupLink::Partial
    }
}

fn link_of_class(class: &str) -> GroupLink {
    match class {
        "linked" => GroupLink::Linked,
        "partial" => GroupLink::Partial,
        _ => GroupLink::Orphan,
    }
}

fn link_label(link: GroupLink) -> &'static str {
    match link {
        GroupLink::Linked => "Enlazado",
        GroupLink::Partial => "Parcial",
        GroupLink::Orphan => "Sin enlace",
    }
}

fn link_class(link: GroupLink) -> &'static str {
    match link {
        GroupLink::Linked => "linked",
        GroupLink::Partial => "partial",
        GroupLink::Orphan => "orphan",
    }
}

fn file_link_label(node: &FileNode) -> &'static str {
    match (node.has_downloads, node.has_media) {
        (true, true) => "En ambos lados",
        (true, false) => "Solo en descargas",
        (false, true) => "Solo en media",
        (false, false) => "Sin enlace",
    }
}

fn files_label(count: usize) -> String {
    if count == 1 {
        "1 fichero".to_string()
    } else {
        format!("{count} ficheros")
    }
}

pub fn format_size(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;
    if gb >= 1.0 {
        format!("{gb:.2} GB")
    } else if mb >= 1.0 {
        format!("{mb:.2} MB")
    } else if kb >= 1.0 {
        format!("{kb:.2} KB")
    } else {
        format!("{bytes} B")
    }
}

fn format_date(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return "—".to_string();
    };
    let Ok(since) = time.duration_since(UNIX_EPOCH) else {
        return "—".to_string();
    };
    chrono::DateTime::from_timestamp(since.as_secs() as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn encode_query(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grouping::GroupKind;
    use std::time::Duration;

    fn sample_node(
        inode: u64,
        size: u64,
        path: &str,
        downloads: bool,
        media: bool,
        modified_secs: Option<u64>,
        seeding: bool,
    ) -> FileNode {
        FileNode {
            key: FileKey { dev: 1, inode },
            size,
            nlink: if downloads && media { 2 } else { 1 },
            paths: vec![path_buf(path)],
            has_downloads: downloads,
            has_media: media,
            torrent_hash: None,
            is_seeding: seeding,
            modified: modified_secs.map(|secs| UNIX_EPOCH + Duration::from_secs(secs)),
        }
    }

    fn path_buf(path: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(path)
    }

    fn group(title: &str, container: &str, side: Side, nodes: Vec<FileNode>) -> Group {
        Group {
            kind: GroupKind::Movie,
            title: title.to_string(),
            media_container: if side == Side::Media {
                Some(path_buf(container))
            } else {
                None
            },
            downloads_container: if side == Side::Downloads {
                Some(path_buf(container))
            } else {
                None
            },
            nodes,
        }
    }

    fn titles(cards: &[Card]) -> Vec<&str> {
        cards.iter().map(|card| card.title.as_str()).collect()
    }

    #[test]
    fn classifies_linked_partial_and_orphan() {
        let linked = group(
            "Inception",
            "/media/Inception",
            Side::Media,
            vec![sample_node(
                1,
                1000,
                "/media/Inception/a.mkv",
                true,
                true,
                Some(10),
                true,
            )],
        );
        let partial = group(
            "Partial",
            "/media/Partial",
            Side::Media,
            vec![
                sample_node(2, 100, "/media/Partial/a.mkv", true, true, Some(20), false),
                sample_node(3, 50, "/media/Partial/b.mkv", false, true, Some(30), false),
            ],
        );
        let orphan = group(
            "Loose",
            "/downloads/Loose",
            Side::Downloads,
            vec![sample_node(
                4,
                500,
                "/downloads/Loose/b.mkv",
                true,
                false,
                Some(5),
                false,
            )],
        );

        let listing = Listing::default();
        let media = cards(&[linked, partial], Side::Media, &listing);
        assert_eq!(titles(&media), ["Inception", "Partial"]);
        assert_eq!(media[0].link_class, "linked");
        assert_eq!(media[1].link_class, "partial");
        assert!(media[0].seeding);

        let orphans = cards(
            &[
                group(
                    "Inception",
                    "/media/Inception",
                    Side::Media,
                    vec![sample_node(
                        1,
                        1000,
                        "/media/Inception/a.mkv",
                        true,
                        true,
                        Some(10),
                        true,
                    )],
                ),
                group(
                    "Partial",
                    "/media/Partial",
                    Side::Media,
                    vec![
                        sample_node(2, 100, "/media/Partial/a.mkv", true, true, Some(20), false),
                        sample_node(3, 50, "/media/Partial/b.mkv", false, true, Some(30), false),
                    ],
                ),
            ],
            Side::Media,
            &Listing {
                filter: Filter::Orphans,
                ..Listing::default()
            },
        );
        assert_eq!(titles(&orphans), ["Partial"]);

        let hardlinked = cards(
            &[
                orphan.clone(),
                group(
                    "Kept",
                    "/downloads/Kept",
                    Side::Downloads,
                    vec![sample_node(
                        9,
                        10,
                        "/downloads/Kept/a.mkv",
                        true,
                        true,
                        Some(1),
                        false,
                    )],
                ),
            ],
            Side::Downloads,
            &Listing {
                filter: Filter::Hardlinked,
                ..Listing::default()
            },
        );
        assert_eq!(titles(&hardlinked), ["Kept"]);
    }

    #[test]
    fn sorts_by_size_and_searches_by_title() {
        let groups = vec![
            group(
                "Alpha",
                "/media/Alpha",
                Side::Media,
                vec![sample_node(
                    1,
                    100,
                    "/media/Alpha/a.mkv",
                    false,
                    true,
                    Some(1),
                    false,
                )],
            ),
            group(
                "Beta",
                "/media/Beta",
                Side::Media,
                vec![sample_node(
                    2,
                    300,
                    "/media/Beta/a.mkv",
                    false,
                    true,
                    Some(50),
                    false,
                )],
            ),
        ];
        let by_size = cards(
            &groups,
            Side::Media,
            &Listing {
                sort: Sort::Size,
                order: Order::Desc,
                ..Listing::default()
            },
        );
        assert_eq!(titles(&by_size), ["Beta", "Alpha"]);

        let found = cards(
            &groups,
            Side::Media,
            &Listing {
                q: "alp".into(),
                ..Listing::default()
            },
        );
        assert_eq!(titles(&found), ["Alpha"]);
    }

    #[test]
    fn sorts_by_date_with_the_newest_file_in_the_group() {
        let groups = vec![
            group(
                "Old",
                "/media/Old",
                Side::Media,
                vec![sample_node(
                    1,
                    10,
                    "/media/Old/a.mkv",
                    false,
                    true,
                    Some(10),
                    false,
                )],
            ),
            group(
                "New",
                "/media/New",
                Side::Media,
                vec![sample_node(
                    2,
                    10,
                    "/media/New/a.mkv",
                    false,
                    true,
                    Some(90),
                    false,
                )],
            ),
        ];
        let by_date = cards(
            &groups,
            Side::Media,
            &Listing {
                sort: Sort::Date,
                order: Order::Desc,
                ..Listing::default()
            },
        );
        assert_eq!(titles(&by_date), ["New", "Old"]);
    }

    #[test]
    fn summary_counts_each_physical_file_once() {
        let nodes = vec![
            sample_node(1, 1000, "/media/Inception/a.mkv", true, true, None, true),
            sample_node(2, 400, "/downloads/Loose/b.mkv", true, false, None, false),
        ];
        let summary = summary(&nodes);
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.saved_bytes, 1000);
        assert_eq!(summary.total_bytes, 1400);
        assert_eq!(summary.orphan_files, 1);
        assert_eq!(summary.seeding_files, 1);
    }

    #[test]
    fn detail_splits_paths_and_looks_up_the_torrent() {
        let mut node = sample_node(
            1,
            2048,
            "/downloads/Inception/a.mkv",
            true,
            true,
            Some(10),
            true,
        );
        node.paths.push(path_buf("/media/movies/Inception/a.mkv"));
        node.torrent_hash = Some("abc".into());
        let group = Group {
            kind: GroupKind::Movie,
            title: "Inception".into(),
            media_container: Some(path_buf("/media/movies/Inception")),
            downloads_container: Some(path_buf("/downloads/Inception")),
            nodes: vec![node],
        };
        let torrents = vec![TorrentInfo {
            hash: "abc".into(),
            name: "Inception.2010".into(),
            state: "stalledUP".into(),
            progress: 1.0,
            ratio: 1.5,
        }];
        let detail = detail(
            &group,
            Path::new("/downloads"),
            &torrents,
            "/library/0",
            "Películas",
        );
        assert_eq!(detail.title, "Inception");
        assert_eq!(detail.link_class, "linked");
        assert_eq!(detail.media_paths.len(), 1);
        assert_eq!(detail.download_paths.len(), 1);
        assert_eq!(detail.torrents[0].name, "Inception.2010");
        assert!(detail.torrents[0].seeding);
        assert_eq!(detail.back_href, "/library/0");
    }

    #[test]
    fn duplicate_nodes_in_a_group_are_counted_once() {
        let node = sample_node(7, 80, "/media/Show/a.mkv", false, true, Some(3), false);
        let group = group("Show", "/media/Show", Side::Media, vec![node.clone(), node]);
        let cards = cards(&[group], Side::Media, &Listing::default());
        assert_eq!(cards[0].files_label, "1 fichero");
        assert_eq!(cards[0].size_label, format_size(80));
    }
}
