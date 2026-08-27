insert into chapter_pages(id, release_id, sort_order, extension)
select page.id, $1, null, page.extension
from unnest($2::uuid[], $3::text[]) as page(id, extension);
