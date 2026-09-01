select cp.id
from chapter_pages cp
join books b on b.id = cp.book_id
where cp.release_id = $1 and cp.sort_order = $2 and (b.visibility = 'Listed' or $3);
