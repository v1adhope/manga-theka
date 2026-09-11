insert into chapter_pages(id, release_id, book_id, sort_order, extension)
select page.id, $1, cr.book_id, null, page.extension
from unnest($2::uuid[], $3::text[]) as page(id, extension)
cross join chapter_releases cr
where cr.id = $1;
