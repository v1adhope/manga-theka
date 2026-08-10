insert into book_titles(id, book_id, language_id, name)
select title.id, $1, title.language_id, title.name
from unnest($2::uuid[], $3::uuid[], $4::text[]) as title(id, language_id, name);
