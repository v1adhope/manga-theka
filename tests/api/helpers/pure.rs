use manga_theka::entity::{
    AlternativeTitle, BookLink, BookQuery, ChapterLocalization, CreatorQuery, Label,
};
use uuid::Uuid;

use super::fakers::LABELS;

pub fn labels(ids: &[Uuid]) -> Vec<Label> {
    ids.iter()
        .map(|id| {
            LABELS
                .iter()
                .find(|l| l.id == *id)
                .expect("fixture label must be seeded")
                .clone()
        })
        .collect()
}

pub fn label_keys(labels: &[Label]) -> Vec<(Uuid, &str, &str)> {
    let mut keys: Vec<(Uuid, &str, &str)> = labels
        .iter()
        .map(|l| (l.id, l.name.as_str(), l.kind.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn day(n: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(n)
}

pub fn rfc3339(at: time::OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .expect("a fixture timestamp must render")
}

pub fn ids(books: &[BookQuery]) -> Vec<Uuid> {
    books.iter().map(|b| b.id).collect()
}

pub fn pick(books: &[BookQuery], wanted: &[usize]) -> Vec<Uuid> {
    wanted.iter().map(|i| books[*i].id).collect()
}

pub fn sorted(mut v: Vec<Uuid>) -> Vec<Uuid> {
    v.sort();

    v
}

pub fn link_keys(links: &[BookLink]) -> Vec<(&str, &str)> {
    let mut keys: Vec<(&str, &str)> = links
        .iter()
        .map(|l| (l.kind.as_ref(), l.url.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn title_keys(titles: &[AlternativeTitle]) -> Vec<(Uuid, &str)> {
    let mut keys: Vec<(Uuid, &str)> = titles
        .iter()
        .map(|t| (t.language_id, t.name.as_ref()))
        .collect();
    keys.sort();

    keys
}

pub fn creator_keys(creators: &[CreatorQuery]) -> Vec<(Uuid, &str, &str, Vec<&str>)> {
    let mut keys: Vec<(Uuid, &str, &str, Vec<&str>)> = creators
        .iter()
        .map(|c| {
            let mut roles: Vec<&str> = c.roles.iter().map(AsRef::as_ref).collect();
            roles.sort();
            (c.id, c.first_name.as_ref(), c.last_name.as_ref(), roles)
        })
        .collect();
    keys.sort();

    keys
}

pub fn localization_keys(localizations: &[ChapterLocalization]) -> Vec<(Uuid, &str)> {
    let mut keys: Vec<(Uuid, &str)> = localizations
        .iter()
        .map(|l| (l.language_id, l.name.as_ref()))
        .collect();
    keys.sort();

    keys
}
