insert into book_links(id, book_id, kind, url)
select link.id, $1, link.kind, link.url
from unnest($2::uuid[], $3::text[], $4::text[]) as link(id, kind, url);
