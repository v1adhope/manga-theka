select book_id, language_id, name
from book_titles
where book_id = any($1::uuid[])
order by book_id, language_id, name;
