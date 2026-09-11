select b.visibility, cr.created_by, cp.id as page_id
from books b
join chapter_releases cr on cr.book_id = b.id
left join chapter_pages cp on cp.id = $2 and cp.release_id = cr.id
where cr.id = $1;
