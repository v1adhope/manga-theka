select b.visibility
from books b
join chapter_pages cp on cp.book_id = b.id
where cp.id = $1 and cp.release_id = $2;
