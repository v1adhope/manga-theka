select id
from book_covers
where book_id = $1
order by id;
