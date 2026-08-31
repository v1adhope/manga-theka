select b.visibility
from books b
join chapters c on c.book_id = b.id
where c.id = $1;
