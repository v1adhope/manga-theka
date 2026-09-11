use manga_theka::entity::{
    AlternativeTitle, BookLink, BookQuery, ChapterLocalization, CreatorQuery, Label, Timestamp,
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

fn sorted_keys<'a, T, K: Ord>(items: &'a [T], to_key: impl FnMut(&'a T) -> K) -> Vec<K> {
    let mut keys: Vec<K> = items.iter().map(to_key).collect();
    keys.sort();

    keys
}

pub fn label_keys(labels: &[Label]) -> Vec<(Uuid, &str, &str)> {
    sorted_keys(labels, |l| (l.id, l.name.as_str(), l.kind.as_ref()))
}

pub fn day(n: i64) -> Timestamp {
    Timestamp::UNIX_EPOCH + time::Duration::days(n)
}

pub fn rfc3339(at: Timestamp) -> String {
    at.into_inner()
        .format(&time::format_description::well_known::Rfc3339)
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
    sorted_keys(links, |l| (l.kind.as_ref(), l.url.as_ref()))
}

pub fn title_keys(titles: &[AlternativeTitle]) -> Vec<(Uuid, &str)> {
    sorted_keys(titles, |t| (t.language_id, t.name.as_ref()))
}

pub fn creator_keys(creators: &[CreatorQuery]) -> Vec<(Uuid, &str, &str, Vec<&str>)> {
    sorted_keys(creators, |c| {
        let mut roles: Vec<&str> = c.roles.iter().map(AsRef::as_ref).collect();
        roles.sort();
        (c.id, c.first_name.as_ref(), c.last_name.as_ref(), roles)
    })
}

pub fn localization_keys(localizations: &[ChapterLocalization]) -> Vec<(Uuid, &str)> {
    sorted_keys(localizations, |l| (l.language_id, l.name.as_ref()))
}
