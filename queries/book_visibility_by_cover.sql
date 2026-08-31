select b.visibility
from books b
join book_covers bc on bc.book_id = b.id
where bc.id = $1;
