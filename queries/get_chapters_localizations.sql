select chapter_id, language_id, name
from chapter_localizations
where chapter_id = any($1::uuid[])
order by chapter_id, language_id, name;
