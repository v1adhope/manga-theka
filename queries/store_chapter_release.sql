insert into chapter_releases(id, chapter_id, language_id, version)
select $1, c.id, $3, 0
from chapters c
join books b on b.id = c.book_id
where c.id = $2 and b.publication_language <> $3
returning id;
