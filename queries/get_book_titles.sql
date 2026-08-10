select id, language_id, name
from book_titles
where book_id = $1
order by id;
