select b.visibility
from books b
join chapter_releases cr on cr.book_id = b.id
where cr.id = $1;
