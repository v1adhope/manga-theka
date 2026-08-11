insert into book_titles(book_id, language_id, name)
select $1, title.language_id, title.name
from unnest($2::uuid[], $3::text[]) as title(language_id, name);
