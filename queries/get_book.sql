select b.id, b.name, b.description, b.publication_year, cr.id as content_rating_id,
       cr.name as content_rating_name, cr.code as content_rating_code, b.status, b.kind,
       l.id as publication_language_id, l.code as publication_language_code,
       l.name as publication_language_name, b.publication_demographic, b.visibility, b.note,
       b.submitted_at, b.updated_at, b.created_at, b.created_by
from books b
join content_ratings cr on cr.id = b.content_rating
join languages l on l.id = b.publication_language
where b.id = $1;
