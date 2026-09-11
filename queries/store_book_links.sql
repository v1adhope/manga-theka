insert into book_links(book_id, kind, url)
select $1, link.kind, link.url
from unnest($2::text[], $3::text[]) as link(kind, url);
