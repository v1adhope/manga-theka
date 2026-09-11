insert into chapter_releases(id, chapter_id, book_id, language_id, version, created_by)
select $1, $2, c.book_id, $3, 1, $4
from chapters c
where c.id = $2;
