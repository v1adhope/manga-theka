insert into chapter_releases(id, chapter_id, book_id, language_id, version)
select $1, $2, c.book_id, $3, 1
from chapters c
where c.id = $2;
